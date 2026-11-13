"""Execute explicit G/D manifest cells through the maintained attempt machinery."""

import argparse
from contextlib import ExitStack, contextmanager
import json
import fcntl
import math
import os
import signal
import subprocess
import sys
from pathlib import Path
import time
from typing import Any

from campaign import run_campaign
from gd_device import _accounting
from gd_resources import GiB, _cleanup_owned_group, _owned_subreaper, aggregate_roots, measure_root, resource_issue, resource_sample, swap_streak
from gd_feasibility import report as feasibility_report
from gd_overlap import analyze_native


def run_manifest(manifest, root, monitor=None, before=None, resources=None, resource_interval: float = 30, load=None, gpu_interval: float = 1):
    """Run supplied cells in order with their actual native artifact identities."""
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True)
    clock_path = root / "campaign-clock.json"
    boot_id = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    if not clock_path.exists():
        clock_path.write_text(json.dumps({"started_ns": time.monotonic_ns(), "boot_id": boot_id,"cell_deadlines":{}}) + "\n")
    clock = json.loads(clock_path.read_text())
    deadline = clock["started_ns"] + int(min(manifest["campaign_seconds"], 14400) * 1_000_000_000)

    swap = {"previous":None,"streak":0}
    cancelled = None
    load = load or (lambda: os.getloadavg()[0])

    def prepare():
        check_clock()
        nonlocal gpu_before
        gpu_before = reservation.enter_context(_gpu_reservation(cell, resources, effective_work_deadline, gpu_interval, stop_manifest))
        samples = []
        if resources:
            samples.append(_sample_resources(resources, effective_work_deadline / 1_000_000_000, start=True, swap=swap))
        check_clock()
        cpu_baseline = _cpu_sample(load) if cell.get("cpu_observation") else None
        quiet = _quiet_start(load, effective_work_deadline, stop_manifest) if cell.get("quiet_start") else None
        check_clock()
        context = before() if before else None
        if quiet is not None:
            context = {"preparation":context,"cpu_quiet":quiet}
        check_clock()
        if resources:
            samples.append(_sample_resources(resources, effective_work_deadline / 1_000_000_000, start=True, swap=swap))
            context = {"preparation":context,"resource_samples":samples}
        if cpu_baseline is not None:
            context = context if isinstance(context,dict) else {"preparation":context}
            context["cpu_baseline"] = cpu_baseline
        check_clock()
        if cell.get("lower"):
            admission = _conditional(cell,root,samples[-1] if samples else None,clock["started_ns"],effective_work_deadline)
            if context is None:
                context = {}
            context["admission"] = admission
        check_clock()
        return context

    def check_clock():
        if cancelled is not None:
            raise _CampaignRefusal("controller_stopped", controller_signal=cancelled)
        if clock["boot_id"] != boot_id:
            raise _CampaignRefusal("campaign_clock_unavailable")
        now = time.monotonic_ns()
        if now >= deadline:
            raise _CampaignRefusal("campaign_time_cap")
        if now >= work_deadline:
            raise _CampaignRefusal("cell_work_deadline")

    def stop_manifest(number):
        nonlocal cancelled
        cancelled = cancelled if cancelled is not None else number

    gpu_before = None
    outcomes = []
    for planned in manifest["cells"]:
        now = time.monotonic_ns()
        hard_deadline = min(deadline, now+int(min(manifest["cell_seconds"],900)*1_000_000_000))
        cell_started_ns = None
        gpu_before = None
        if clock["boot_id"] == boot_id and now < deadline:
            cell_deadlines = clock["cell_deadlines"]
            if planned["id"] not in cell_deadlines:
                cell_deadlines[planned["id"]] = {"entered_ns":now,"hard_deadline_ns":hard_deadline}
                clock_path.write_text(json.dumps(clock) + "\n")
            extent = cell_deadlines[planned["id"]]
            cell_started_ns = extent["entered_ns"]
            hard_deadline = min(hard_deadline, extent["hard_deadline_ns"])
        work_deadline = hard_deadline-30_000_000_000
        bounds = {"work_deadline_ns": work_deadline, "hard_deadline_ns": hard_deadline,
                  "shared_deadline_ns": deadline}
        bounds = _effective_bounds(planned,bounds)
        effective_work_deadline = bounds["work_deadline_ns"]

        def execute(cell, command, config, attempt, observe):
            nonlocal cancelled
            native = dict(cell,campaign_root=str(root)) if cell.get("template_from") else cell
            result = _run_native(native, command, config, attempt, observe, bounds, resources, resource_interval, swap, load, gpu_before, gpu_interval)
            if cell_started_ns is not None:
                result["cell_started_ns"] = cell_started_ns
            cancelled = result.get("controller_signal")
            return result

        artifact = manifest["artifacts"][planned["artifact"]]
        cell = dict(planned, artifact_identity=artifact)
        command = ([sys.executable,str(Path(__file__).with_name("gd_stage.py"))]
                   if (cell["family"]=="G_PREPARE" or cell.get("template_from")
                       or ("n" in cell and (cell["family"]=="D_HELD_FIXTURE"
                           or (cell["family"]=="D_QUERY_FIXTURE" and "write_records" not in cell))))
                   else [artifact["path"]])
        try:
            with ExitStack() as reservation:
                outcomes.extend(run_campaign([cell], root, command,
                                             timeout=manifest["cell_seconds"], completion=_native_complete,
                                             monitor=monitor, before=prepare,
                                             poll_interval=0.1, observe_start=monitor is not None, execute=execute))
        except _CampaignRefusal as error:
            if error.controller_signal is not None:
                stop_manifest(error.controller_signal)
            outcome = {"id": cell["id"], "cell": cell, "status": "not_run_resource",
                       "reason": str(error), "pid": None, "attempt": None, "exit": None}
            if cancelled is not None:
                outcome["controller_signal"] = cancelled
            if error.admission is not None:
                outcome["admission"] = error.admission
            state_path = root / "campaign-state.json"
            previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
            previous[cell["id"]] = outcome
            state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
            outcomes.append(outcome)
    return outcomes


@contextmanager
def _gpu_reservation(cell, resources, work_deadline, interval, on_stop):
    if not cell.get("gpu_reservation"):
        yield
        return
    gpu = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b"
    if cell.get("gpu_uuid") != gpu:
        raise _CampaignRefusal("gpu_identity_unapproved")
    task = Path(resources["task"] if resources else os.environ["S30_GD_TASK"])
    directory = task/"locks"
    directory.mkdir(exist_ok=True)
    with (directory/(gpu+".lock")).open("a") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise _CampaignRefusal("gpu_reserved") from error
        if os.environ.get("CUDA_VISIBLE_DEVICES") != gpu or os.environ.get("FATHOMDB_EMBED_DEVICE") != "cuda:0":
            raise _CampaignRefusal("gpu_environment_unapproved")
        samples = []
        with _owned_stop_flags(on_stop) as stopped:
            while True:
                if stopped["signal"] is not None:
                    raise _CampaignRefusal("controller_stopped", {"gpu_samples":samples}, stopped["signal"])
                if time.monotonic_ns() >= work_deadline:
                    raise _CampaignRefusal("gpu_idle_deadline", {"gpu_samples":samples})
                try:
                    before = _accounting(None, min(work_deadline/1_000_000_000,time.monotonic()+5))
                except Exception as error:
                    raise _CampaignRefusal(f"gpu_observation_unavailable: {error}", {"gpu_samples":samples}) from error
                samples.append(before)
                if stopped["signal"] is not None:
                    raise _CampaignRefusal("controller_stopped", {"gpu_samples":samples}, stopped["signal"])
                if time.monotonic_ns() >= work_deadline:
                    raise _CampaignRefusal("gpu_idle_deadline", {"gpu_samples":samples})
                if before["util"] == 0 and before["total_MiB"]-before["used_MiB"] >= 8192:
                    break
                time.sleep(max(0,min(max(0.1,interval),(work_deadline-time.monotonic_ns())/1_000_000_000)))
        yield samples


def _cpu_sample(load):
    try:
        value = float(load())
        if not math.isfinite(value) or value < 0:
            raise ValueError("invalid one-minute CPU load")
    except Exception as error:
        raise _CampaignRefusal(f"cpu_load_unavailable: {error}") from error
    return {"observed_ns":time.monotonic_ns(),"load1":value}


def _quiet_start(load, deadline_ns, on_stop):
    observations = []
    with _owned_stop_flags(on_stop) as stopped:
        while True:
            if stopped["signal"] is not None:
                raise _CampaignRefusal("controller_stopped", {"cpu_quiet":observations}, stopped["signal"])
            remaining = (deadline_ns-time.monotonic_ns())/1_000_000_000
            if remaining <= 0:
                raise _CampaignRefusal("cpu_quiet_deadline", {"cpu_quiet":observations})
            sample = _cpu_sample(load)
            value = sample["load1"]
            observations.append(sample)
            if stopped["signal"] is not None:
                raise _CampaignRefusal("controller_stopped", {"cpu_quiet":observations}, stopped["signal"])
            if observations[-1]["observed_ns"] >= deadline_ns:
                raise _CampaignRefusal("cpu_quiet_deadline", {"cpu_quiet":observations})
            if value <= 2:
                return observations
            time.sleep(max(0, min(0.1, (deadline_ns-time.monotonic_ns())/1_000_000_000)))


def _conditional(cell,root,current,shared_start,work_deadline):
    state_path=root/"campaign-state.json"
    state={row["id"]:row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
    lower=state.get(cell["lower"])
    if (lower is None or lower["status"]!="complete" or current is None
            or not lower.get("cell_elapsed_ns") or not lower["cell"].get("n")):
        raise _CampaignRefusal("lower_observation_unavailable")
    if cell.get("requires_overlap"):
        path=Path(lower["attempt"])/"qualification.json"
        if lower["cell"].get("observer") is not True or not path.exists():
            raise _CampaignRefusal("lower_overlap_unqualified")
        qualification=json.loads(path.read_text())
        if qualification.get("status")!="OBSERVED_OVERLAP" or not qualification.get("witnesses"):
            raise _CampaignRefusal("lower_overlap_unqualified")
    raw="g_raw" if cell["family"].startswith("G") else "d_raw"
    observed=lower.get("resource_samples") or []
    if not observed or any(raw not in sample for sample in observed) or raw not in current:
        raise _CampaignRefusal("lower_storage_observation_unavailable")
    try:
        retained=aggregate_roots([measure_root(Path(lower["attempt"]).parent,
                                              deadline=min(work_deadline/1_000_000_000,time.monotonic()+5))])
    except Exception as error:
        raise _CampaignRefusal(f"lower_storage_observation_unavailable: {error}") from error
    transient=max(0,max(sample[raw] for sample in observed)-observed[-1][raw])
    report: dict[str,Any]=feasibility_report({"lower_n":lower["cell"]["n"],"target_n":cell["n"],
        "elapsed_ns":lower["cell_elapsed_ns"],"peak_bytes":retained+transient,
        "retained_family_bytes":current[raw],"lower_status":"complete",
        "shared_elapsed_ns":time.monotonic_ns()-shared_start})
    remaining=max(0,work_deadline-time.monotonic_ns())
    eligible=report["admission"]["eligible"] and report["time"]["projected_ns"]<=remaining
    report.update(lower=cell["lower"],
        storage_scope="retained cell/all attempts plus sampled transient family excess; not a continuous peak",
        conditional_gate={"eligible":eligible,"remaining_work_ns":remaining})
    if not eligible:
        raise _CampaignRefusal("conditional_not_feasible",report)
    return report


def _sample_resources(resources, deadline, start=False, swap=None):
    try:
        sample = resource_sample(resources["roots"], Path(resources["task"]), resources["baseline"],
                                 deadline=deadline)
    except Exception as error:
        raise _CampaignRefusal(f"resource_observation_unavailable: {error}") from error
    sample["observed_ns"] = time.monotonic_ns()
    sample["baseline"] = resources["baseline"]
    issue = resource_issue(sample, 200*GiB, start=start)
    if swap is not None:
        observed = dict(sample,time=sample["observed_ns"]/1_000_000_000)
        previous = swap["previous"]
        if previous is None:
            swap["previous"] = observed
        elif observed["time"]-previous["time"] >= 30:
            swap["streak"], swap_issue = swap_streak(observed,previous,swap["streak"])
            swap["previous"] = observed
            issue = issue or swap_issue
    if issue:
        raise _CampaignRefusal(issue)
    return sample


@contextmanager
def _owned_stop_flags(on_stop=None):
    state: dict[str, int | None] = {"signal":None}
    previous = {number:signal.getsignal(number) for number in (signal.SIGTERM,signal.SIGINT)}

    def stop(number, _frame):
        if state["signal"] is None:
            state["signal"] = number
            if on_stop is not None:
                on_stop(number)

    try:
        for number in previous:
            signal.signal(number,stop)
        yield state
    finally:
        for number, handler in previous.items():
            signal.signal(number,handler)


def _effective_bounds(cell,bounds):
    effective={key:min(cell.get(key,limit),limit) for key,limit in bounds.items()}
    hard=min(effective["hard_deadline_ns"],effective["shared_deadline_ns"])
    effective["hard_deadline_ns"]=hard
    effective["work_deadline_ns"]=min(effective["work_deadline_ns"],max(0,hard-30_000_000_000))
    return effective


def _run_completion(command,attempt,bounds,stopped):
    """Run the actual validator command and retain its processing outcome."""
    started=time.monotonic_ns()
    with _owned_subreaper(), (attempt/"validation.log").open("wb") as log:
        child=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
        try:
            code=child.wait()
        finally:
            cleanup=_cleanup_owned_group(child,deadline=min(bounds["hard_deadline_ns"]/1e9,time.monotonic()+20))
    return {"complete":code==0 and cleanup["complete"],"pid":child.pid,"child_exit":code,
            "cleanup":cleanup,"elapsed_ns":time.monotonic_ns()-started,
            "error":None if code==0 else "native completion rejected"}


def _run_native(cell, command, config, attempt, monitor, bounds, resources=None, resource_interval: float = 30, swap=None, load=None, gpu_before=None, gpu_interval: float = 1):
    native = dict(cell,**_effective_bounds(cell,bounds))
    hard_ns = native["hard_deadline_ns"]
    config.write_text(json.dumps(native, indent=2) + "\n")
    refusal = {"exit":125, "error":"native_prelaunch_deadline", "pid":None, "pgid":None,
               "child_exit":None, "cleanup":{"complete":True,"not_started":True}}
    if time.monotonic_ns() >= native["work_deadline_ns"]:
        return refusal
    work = native["work_deadline_ns"] / 1_000_000_000
    hard = hard_ns / 1_000_000_000
    error = None
    samples = []
    next_probe = time.monotonic()+resource_interval
    cpu_samples = []
    next_cpu_probe = time.monotonic()
    gpu_samples: list[dict[str,Any]] = [dict(row,phase="before") for row in gpu_before] if gpu_before is not None else []
    next_gpu_probe = time.monotonic()
    with _owned_stop_flags() as stopped, _owned_subreaper(), (attempt / "child.log").open("wb") as log:
        if stopped["signal"] is not None:
            number = stopped["signal"]
            return dict(refusal,exit=128+number,error=f"controller_signal_{number}",controller_signal=number)
        if time.monotonic_ns() >= native["work_deadline_ns"]:
            return refusal
        child = subprocess.Popen([*command, str(config), str(attempt)], stdout=log,
                                 stderr=subprocess.STDOUT, start_new_session=True)
        try:
            while True:
                if stopped["signal"] is not None:
                    error = f"controller_signal_{stopped['signal']}"
                    break
                now = time.monotonic()
                if now >= work:
                    error = "cell work deadline"
                    break
                if child.poll() is not None:
                    break
                if native.get("gpu_reservation") and now >= next_gpu_probe:
                    try:
                        sample = _accounting(child.pid,min(work,time.monotonic()+5))
                        gpu_samples.append(dict(sample,phase="running"))
                    except Exception as failure:
                        error = f"gpu_observation_unavailable: {failure}"
                        break
                    next_gpu_probe = time.monotonic()+max(0.1,gpu_interval)
                if native.get("cpu_observation") and now >= next_cpu_probe:
                    try:
                        sample = _cpu_sample(load)
                        sample.update(phase="running",child_alive_at_start=True,
                                      includes_intentional_worker_load=native["family"].startswith("D_"),
                                      above_noisy_threshold=sample["load1"]>2.5)
                        cpu_samples.append(sample)
                    except _CampaignRefusal as refusal:
                        error = str(refusal)
                        break
                    next_cpu_probe = time.monotonic()+1
                if resources and now >= next_probe:
                    probe_started_ns = time.monotonic_ns()
                    alive = child.poll() is None
                    if not alive:
                        break
                    try:
                        sample = _sample_resources(resources, work, swap=swap)
                        sample.update(phase="running", native_pid=child.pid,
                                      child_alive_at_start=alive, probe_started_ns=probe_started_ns)
                        samples.append(sample)
                    except _CampaignRefusal as refusal:
                        error = str(refusal)
                        break
                    next_probe = time.monotonic()+resource_interval
                if stopped["signal"] is not None:
                    error = f"controller_signal_{stopped['signal']}"
                    break
                if monitor:
                    error = monitor(child.pid)
                    if error:
                        break
                remaining = work-time.monotonic()
                if remaining <= 0:
                    error = "cell work deadline"
                    break
                try:
                    child.wait(timeout=min(0.1, remaining))
                except subprocess.TimeoutExpired:
                    pass
            if monitor and error is None and stopped["signal"] is None:
                error = monitor(child.pid)
            if time.monotonic() >= work:
                error = error or "cell work deadline"
        finally:
            if child.poll() is None and stopped["signal"] is None:
                cooperative = min(time.monotonic()+5, hard-25)
                if cooperative > time.monotonic():
                    try:
                        child.wait(timeout=cooperative-time.monotonic())
                    except subprocess.TimeoutExpired:
                        pass
            cleanup = _cleanup_owned_group(child, deadline=min(hard-5, time.monotonic()+20))
        if not cleanup["complete"]:
            error = error or "owned group cleanup incomplete"
        if resources and stopped["signal"] is None:
            try:
                sample = _sample_resources(resources, hard, swap=swap)
                sample["phase"] = "final"
                samples.append(sample)
            except _CampaignRefusal as refusal:
                error = error or str(refusal)
        if native.get("gpu_reservation") and stopped["signal"] is None:
            try:
                gpu_samples.append(dict(_accounting(child.pid,min(hard,time.monotonic()+5)),phase="final"))
            except Exception as failure:
                error = error or f"gpu_observation_unavailable: {failure}"
            running=[row for row in gpu_samples if row["phase"]=="running"]
            modes=native.get("passes",[native.get("mode","")])
            inference=(native["family"] in {"G_PREPARE","G_TEMPLATE","GG_CELL"}
                       or any(mode.endswith(("_hybrid","_mean")) for mode in modes))
            if not any(row["own_MiB"]>0 and (row["util"]>0 or not inference) for row in running):
                error = error or ("gpu_engagement_unobserved" if inference else "gpu_process_unobserved")
        if native.get("cpu_observation") and stopped["signal"] is None:
            try:
                sample = _cpu_sample(load)
                sample.update(phase="final",child_alive_at_start=False,
                              includes_intentional_worker_load=native["family"].startswith("D_"),
                              above_noisy_threshold=sample["load1"]>2.5)
                cpu_samples.append(sample)
            except _CampaignRefusal as refusal:
                error = error or str(refusal)
        if time.monotonic() >= hard:
            error = error or "cell hard deadline"
        stop_signal = stopped["signal"]
        if stop_signal is not None:
            error = error or f"controller_signal_{stop_signal}"
        validation=None
        if child.returncode==0 and error is None and stopped["signal"] is None:
            validation=_run_completion([sys.executable,str(Path(__file__).resolve()),"--validate",str(config),str(attempt)],attempt,native,stopped)
            if not validation["complete"]:
                error=validation["error"] or "native completion unverified"
        return {"exit":128+stop_signal if stop_signal is not None else (124 if error else child.returncode), "error":error,
                "pid":child.pid, "pgid":child.pid, "child_exit":child.returncode, "cleanup":cleanup,
                "complete":validation is not None and validation["complete"],"validation":validation,
                "resource_samples":samples if resources else None, "controller_signal":stop_signal,
                "cpu_samples":cpu_samples if native.get("cpu_observation") else None,
                "gpu_samples":gpu_samples if native.get("gpu_reservation") else None}


class _CampaignRefusal(Exception):
    def __init__(self,reason,admission=None,controller_signal=None):
        super().__init__(reason)
        self.admission=admission
        self.controller_signal=controller_signal


def _native_complete(cell, attempt):
    if cell["family"] == "G_PREPARE":
        metadata = attempt / "case-0/template.json"
        questions = attempt / "questions.jsonl"
        if not metadata.exists() or not questions.exists():
            return False
        proof = json.loads(metadata.read_text()).get("preparation",{})
        coverage = proof.get("coverage",{})
        return (proof.get("api_complete") is True and proof.get("write_rows")==cell["n"]
                and proof.get("pending")==0 and len(coverage.get("rows",[]))==cell["n"]
                and coverage.get("profile",{}).get("dimension")==384
                and len(questions.read_text().splitlines())==cell.get("query_count",500))
    if cell["family"] in {"G1_PLAN_FIXTURE", "G1_CELL"}:
        base = attempt / "cell" if cell["family"] == "G1_PLAN_FIXTURE" else attempt
        passes = cell["passes"]
        questions = attempt / "questions.jsonl" if cell.get("queries_from") else Path(cell["queries"])
        if not questions.exists():
            return False
        query_count = len(questions.read_text().splitlines())
        warmups = cell.get("warmups", 0)
        for ordinal, mode in enumerate(passes):
            path = base / f"pass-{ordinal}" / "api-proof.json"
            if not path.exists():
                return False
            proof = json.loads(path.read_text())
            if (proof.get("mode"), proof.get("warmup_api_calls"), proof.get("timed_api_calls"),
                    proof.get("api_calls")) != (mode, warmups, query_count, warmups+query_count):
                return False
        return bool(passes)
    if cell["family"]=="GG_CELL":
        control=attempt/"gg-control.json"
        if not control.exists():
            return False
        proof=json.loads(control.read_text())
        if (proof.get("api_complete"),proof.get("base_rows"),proof.get("pending"),proof.get("selected_cuda_uuid"))!=(True,cell["n"],0,cell["gpu_uuid"]):
            return False
        stages=[("time","baseline",100),("time","before",80),("time","inside",100),
                ("time","after",120),("governed","valid",100),("governed","deleted",100)]
        for family,stage,as_of in stages:
            path=attempt/family/stage/"api-proof.json"
            if not path.exists():
                return False
            proof=json.loads(path.read_text())
            if (proof.get("stage"),proof.get("valid_as_of"),proof.get("warmup_calls"),proof.get("timed_calls"),proof.get("api_calls"))!=(stage,as_of,cell["warmups"],cell["samples"],cell["warmups"]+cell["samples"]):
                return False
        return True
    if cell["family"]=="D_HELD_FIXTURE":
        path=attempt/"controls.json"
        if not path.exists():
            return False
        proof=json.loads(path.read_text())
        count=cell.get("n",len(cell.get("records",[])))
        coverage=proof.get("coverage",{})
        held=proof.get("held",{})
        released=proof.get("released",{})
        if (proof.get("api_complete") is not True or proof.get("release_requests")!=1
                or held.get("readiness",{}).get("pending_count")!=count
                or held.get("sql",{}).get("physical_vectors")!=0
                or released.get("readiness",{}).get("pending_count")!=0
                or released.get("sql",{}).get("physical_vectors")!=count
                or coverage.get("verified") is not True or len(coverage.get("rows",[]))!=count
                or coverage.get("profile",{}).get("dimension")!=768):
            return False
        if cell.get("queries") or cell.get("query_source"):
            calls=[json.loads(line) for line in (attempt/"queries.jsonl").read_text().splitlines()]
            return ([row["phase"] for row in calls]==["warmup"]*cell["warmups"]+["timed"]*cell["samples"]
                    and all(row.get("ok") is True for row in calls))
        return True
    name = "complete.json" if cell["family"] == "D_QUERY_FIXTURE" else "result.json"
    path = attempt / name
    complete=path.exists() and json.loads(path.read_text()).get("complete") is True
    if complete and cell["family"]=="D_QUERY_FIXTURE" and cell.get("workload")=="overlap":
        (attempt/"qualification.json").write_text(json.dumps(analyze_native(attempt),indent=2)+"\n")
    return complete


def main():
    """Execute an explicit manifest and print actual per-cell outcomes."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--validate",action="store_true",help=argparse.SUPPRESS)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text())
    if args.validate:
        return 0 if _native_complete(manifest,args.output) else 1
    outcomes = run_manifest(manifest, args.output, resources=manifest.get("resources"))
    print(json.dumps(outcomes, indent=2))
    stopped = next((row["controller_signal"] for row in outcomes if row.get("controller_signal") is not None),None)
    return 128+stopped if stopped is not None else int(any(row["status"] != "complete" for row in outcomes))


if __name__ == "__main__":
    raise SystemExit(main())
