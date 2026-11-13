"""Execute explicit G/D manifest cells through the maintained attempt machinery."""

import argparse
from contextlib import contextmanager
import json
import signal
import subprocess
from pathlib import Path
import time

from campaign import run_campaign
from gd_resources import GiB, _cleanup_owned_group, _owned_subreaper, resource_issue, resource_sample, swap_streak


def run_manifest(manifest, root, monitor=None, before=None, resources=None, resource_interval: float = 30):
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

    def prepare():
        check_clock()
        samples = []
        if resources:
            samples.append(_sample_resources(resources, work_deadline / 1_000_000_000, start=True, swap=swap))
        check_clock()
        context = before() if before else None
        check_clock()
        if resources:
            samples.append(_sample_resources(resources, work_deadline / 1_000_000_000, start=True, swap=swap))
            context = {"preparation":context,"resource_samples":samples}
        check_clock()
        return context

    def check_clock():
        if cancelled is not None:
            raise _CampaignRefusal("controller_stopped")
        if clock["boot_id"] != boot_id:
            raise _CampaignRefusal("campaign_clock_unavailable")
        now = time.monotonic_ns()
        if now >= deadline:
            raise _CampaignRefusal("campaign_time_cap")
        if now >= work_deadline:
            raise _CampaignRefusal("cell_work_deadline")

    outcomes = []
    for planned in manifest["cells"]:
        now = time.monotonic_ns()
        hard_deadline = min(deadline, now+int(min(manifest["cell_seconds"],900)*1_000_000_000))
        if clock["boot_id"] == boot_id and now < deadline:
            cell_deadlines = clock["cell_deadlines"]
            if planned["id"] not in cell_deadlines:
                cell_deadlines[planned["id"]] = hard_deadline
                clock_path.write_text(json.dumps(clock) + "\n")
            hard_deadline = min(hard_deadline, cell_deadlines[planned["id"]])
        work_deadline = hard_deadline-30_000_000_000
        bounds = {"work_deadline_ns": work_deadline, "hard_deadline_ns": hard_deadline,
                  "shared_deadline_ns": deadline}

        def execute(cell, command, config, attempt, observe):
            nonlocal cancelled
            result = _run_native(cell, command, config, attempt, observe, bounds, resources, resource_interval, swap)
            cancelled = result.get("controller_signal")
            return result

        artifact = manifest["artifacts"][planned["artifact"]]
        cell = dict(planned, artifact_identity=artifact)
        try:
            outcomes.extend(run_campaign([cell], root, [artifact["path"]],
                                         timeout=manifest["cell_seconds"], completion=_native_complete,
                                         monitor=monitor, before=prepare,
                                         poll_interval=0.1, observe_start=monitor is not None, execute=execute))
        except _CampaignRefusal as error:
            outcome = {"id": cell["id"], "cell": cell, "status": "not_run_resource",
                       "reason": str(error), "pid": None, "attempt": None, "exit": None}
            state_path = root / "campaign-state.json"
            previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
            previous[cell["id"]] = outcome
            state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
            outcomes.append(outcome)
    return outcomes


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
def _owned_stop_flags():
    state: dict[str, int | None] = {"signal":None}
    previous = {number:signal.getsignal(number) for number in (signal.SIGTERM,signal.SIGINT)}

    def stop(number, _frame):
        if state["signal"] is None:
            state["signal"] = number

    try:
        for number in previous:
            signal.signal(number,stop)
        yield state
    finally:
        for number, handler in previous.items():
            signal.signal(number,handler)


def _run_native(cell, command, config, attempt, monitor, bounds, resources=None, resource_interval: float = 30, swap=None):
    native = dict(cell)
    for key, limit in bounds.items():
        native[key] = min(native.get(key, limit), limit)
    hard_ns = min(native["hard_deadline_ns"], native["shared_deadline_ns"])
    native["work_deadline_ns"] = min(native["work_deadline_ns"], max(0, hard_ns-30_000_000_000))
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
        if time.monotonic() >= hard:
            error = error or "cell hard deadline"
        stop_signal = stopped["signal"]
        if stop_signal is not None:
            error = error or f"controller_signal_{stop_signal}"
        return {"exit":128+stop_signal if stop_signal is not None else (124 if error else child.returncode), "error":error,
                "pid":child.pid, "pgid":child.pid, "child_exit":child.returncode, "cleanup":cleanup,
                "resource_samples":samples if resources else None, "controller_signal":stop_signal}


class _CampaignRefusal(Exception):
    pass


def _native_complete(cell, attempt):
    if cell["family"] in {"G1_PLAN_FIXTURE", "G1_CELL"}:
        base = attempt / "cell" if cell["family"] == "G1_PLAN_FIXTURE" else attempt
        passes = cell["passes"]
        query_count = len(Path(cell["queries"]).read_text().splitlines())
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
    name = "complete.json" if cell["family"] == "D_QUERY_FIXTURE" else "result.json"
    path = attempt / name
    return path.exists() and json.loads(path.read_text()).get("complete") is True


def main():
    """Execute an explicit manifest and print actual per-cell outcomes."""
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text())
    outcomes = run_manifest(manifest, args.output, resources=manifest.get("resources"))
    print(json.dumps(outcomes, indent=2))
    stopped = next((row["controller_signal"] for row in outcomes if row.get("controller_signal") is not None),None)
    return 128+stopped if stopped is not None else int(any(row["status"] != "complete" for row in outcomes))


if __name__ == "__main__":
    raise SystemExit(main())
