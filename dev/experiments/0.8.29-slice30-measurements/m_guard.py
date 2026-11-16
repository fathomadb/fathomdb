"""Named-phase outer guardian; shared charges survive controller death and owned cleanup."""

import fcntl
import json
import os
import signal
from pathlib import Path
import subprocess
import time
import uuid

from g_attempt import live_owned_pids, stop_owned_group, unreaped_exit
from g_boundary import TaskBoundary
from g_campaign import active_bytes
from g_resource import resource_sample
from m_load import observe_load, summarize_load
from m_record import BoundedLog, write_guardian


def write_budget(path, value):
    """Publish one complete shared-ledger observation atomically."""
    path = Path(path)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    os.replace(temporary, path)


def process_start(pid):
    """Read the Linux start tick that distinguishes an owned process identity."""
    return int(Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()[19])


class Accounting:
    """Track residual outer wall without charging an active/committed cell twice."""

    def __init__(self, path, started_ns, *, phase="M"):
        if phase not in ("M", "D", "S50"):
            raise ValueError("unknown shared guardian phase")
        self.phase, self.guardian_key = phase, phase.lower()+"_guardian"
        self.path, self.started_ns = Path(path), started_ns
        prior = self.read()
        self.prior_used = prior["used_ns"]
        self.prior_cells = dict(prior.get("cell_used_ns", {}))
        self.calibration = None

    def begin_calibration(self, limits, root, value, *, cell_id=None):
        """Pin one cumulative allowance before qualification or resource preflight."""
        if self.phase == "S50":
            from g_pair import CALIBRATION_ID, admit, validate_limits
        else:
            from m_calibration import CALIBRATION_ID, admit, admit_selected, validate_limits
        validate_limits(limits)
        usage_key = "calibration_usage" if cell_id is None else "selected_cell_usage"
        identity = CALIBRATION_ID if cell_id is None else cell_id
        usage = value.setdefault(usage_key, {}).get(identity, {})
        if usage and usage.get("limits") != limits:
            raise ValueError("selected cumulative calibration allowance changed")
        roots = usage.get("roots", [])
        prior_logs = usage.get("invocation_logs", [])
        current_logs = ([str(root.parent / (root.name + suffix)) for suffix in
            ("-service.log", "-supervisor.log", "-samples.jsonl")] if cell_id is not None else [])
        if usage:
            usage["new_retained_bytes"] = max(usage["new_retained_bytes"],
                                             sum(active_bytes(path) for path in [*roots, *prior_logs]))
        prior_ns, prior_bytes = usage.get("wall_ns", 0), usage.get("new_retained_bytes", 0)
        self.calibration = dict(id=CALIBRATION_ID, prior_wall_ns=prior_ns, prior_retained_bytes=prior_bytes,
                                limits=dict(limits), root=str(root),
                                baseline_bytes=active_bytes(self.path.parent)-sum(active_bytes(path) for path in current_logs), peak_growth=0,
                                usage_key=usage_key, cell_id=identity)
        value[usage_key][identity] = dict(usage,
            limits=dict(limits), wall_ns=prior_ns,
            new_retained_bytes=prior_bytes, roots=list(dict.fromkeys([*roots, str(root)])))
        if cell_id is not None:
            value[usage_key][identity]["invocation_logs"] = list(dict.fromkeys([*prior_logs, *current_logs]))
        remaining = (admit(self.profile, value, active_bytes(self.path.parent), limits) if cell_id is None
                     else admit_selected(self.profile, value, active_bytes(self.path.parent), limits, cell_id))
        self.calibration.update(remaining)
        return self.started_ns + remaining["remaining_phase_ns"]

    def growth(self):
        """Include all allocation growth, retaining the peak if files are shortened."""
        stage = self.calibration
        stage["peak_growth"] = max(stage["peak_growth"],
            active_bytes(self.path.parent) - stage["baseline_bytes"])
        return stage["peak_growth"]

    def read(self):
        """Read the existing shared allocation; never create/reset usage."""
        value = json.loads(self.path.read_text())
        if value.get("schema") != "slice30-gd-budget/v1":
            raise ValueError("unknown shared ledger")
        return value

    def uncommitted(self, value, now):
        """Return only the actual active interval not already committed to that cell."""
        active = value.get("inflight")
        if not active or active.get("status") == "guardian_stopped":
            return 0
        origin = active["started_monotonic_ns"]
        if type(origin) is not int or origin < self.started_ns:
            raise ValueError(f"active {self.phase} interval does not belong to this guardian")
        committed = (value.get("cell_used_ns", {}).get(active["cell"], 0)
                     - self.prior_cells.get(active["cell"], 0))
        return max(0, now - origin - committed)

    def total(self, now):
        """Include all outer work and all original shared charges while running."""
        value = self.read()
        return max(value["used_ns"], self.prior_used + now - self.started_ns)

    def finish(self, *, uncertain, stopped_ns):
        """Charge actual outer elapsed time after the original owned cleanup result."""
        value = self.read()
        now = time.monotonic_ns()
        active = value.get("inflight")
        recovered = self.uncommitted(value, now if uncertain else stopped_ns)
        if active:
            usage = value.setdefault("cell_used_ns", {})
            usage[active["cell"]] = usage.get(active["cell"], 0) + recovered
            value["used_ns"] += recovered
            active.update(status="cleanup_uncertain" if uncertain else "guardian_stopped",
                          accounted_through_monotonic_ns=now if uncertain else stopped_ns,
                          quiescence_known=not uncertain)
        residual = max(0, now - self.started_ns - (value["used_ns"] - self.prior_used))
        value["used_ns"] += residual
        common = value.setdefault("common_used_ns", {})
        common[self.phase] = common.get(self.phase, 0) + residual
        if uncertain:
            value[self.guardian_key]["status"] = "cleanup_uncertain"
        else:
            value[self.guardian_key] = None
        if self.calibration:
            stage = self.calibration
            usage = value[stage["usage_key"]][stage["cell_id"]]
            usage["wall_ns"] = stage["prior_wall_ns"] + value["used_ns"] - self.prior_used
            usage["new_retained_bytes"] = stage["prior_retained_bytes"] + self.growth()
        write_budget(self.path, value)
        return {"common_ns": residual, "recovered_cell_ns": recovered,
                "charged_ns": value["used_ns"] - self.prior_used}


def _limits(profile, phase):
    if phase == "D" and any(type(profile.get(key)) is not int or not 0 < profile[key] <= bound
            for key, bound in (("total_shared_ns", 48*3600*10**9),
                               ("total_shared_retained_bytes", 192*1024**3))):
        raise ValueError("D shared maxima are48h/192GiB")
    default = 0 if phase == "M" else None
    reserved = (profile.get("reserved_other_work_ns", default),
                profile.get("reserved_other_work_bytes", default))
    if any(type(value) is not int or value < 0 for value in reserved):
        raise ValueError("bounded other-work reservation unavailable")
    limits = profile["total_shared_ns"]-reserved[0], profile["total_shared_retained_bytes"]-reserved[1]
    if any(type(value) is not int or value <= 0 for value in limits):
        raise RuntimeError(f"shared reserve leaves no positive {phase} allowance")
    return limits


def _guard(command, *, allocation, profile, root, task, admission=False, common_limit_ns=None,
          started_ns=None, phase="M", calibration=None, primary_limits=None, cell_allowance=None, cancelled=None, diagnostic_memory=False):
    """Own the whole controller phase, private boundary, resource floors and final charge."""
    cancelled = cancelled if cancelled is not None else [False]
    if diagnostic_memory and phase != "D":
        raise ValueError("private diagnostic memory capture belongs only to D")
    if phase not in ("M", "D", "S50"):
        raise ValueError("unknown shared guardian phase")
    guardian_key = phase.lower()+"_guardian"
    began = time.monotonic_ns() if started_ns is None else started_ns
    allocation, root, task = map(lambda p: Path(p).resolve(), (allocation, root, task))
    schema = "slice50-pair-budget/v1" if phase == "S50" else f"slice30-{phase.lower()}-budget/v1"
    whole_cell = profile.get("whole_cell_ns")
    whole_valid = whole_cell == 900 * 10**9
    if phase == "D":
        from d_controller import cell_limit_ns
        ceiling = max(cell_limit_ns(n, maximum=True) for n in (10000, 100000, 300000))
        whole_valid = type(whole_cell) is int and 0 < whole_cell <= ceiling
    if (profile.get("schema") != schema or profile.get("status") != "APPROVED"
            or not whole_valid
            or Path(profile["ledger"]).resolve() != allocation / "g-budget.json"
            or not root.is_relative_to(allocation)):
        raise ValueError(f"approved pinned shared profile and owned {phase} allocation required")
    if cell_allowance is not None and (phase != "M" or calibration is not None or primary_limits is not None):
        raise ValueError("select calibration or one required M cell")
    bounded = calibration if cell_allowance is None else cell_allowance["limits"]
    accounting = Accounting(allocation / "g-budget.json", began, phase=phase) if phase in ("M", "S50") else None
    limits = _limits(profile, phase) if phase in ("M", "S50") else None
    if bounded is not None and phase not in ("M", "S50"):
        raise ValueError("calibration is an M-only stage")
    if primary_limits is not None:
        if phase != "S50" or calibration is not None:
            raise ValueError("finite primary stops belong only to the private S50 primary scope")
        from g_pair_freeze import primary_limits as require_primary_limits
        primary_limits = require_primary_limits({"primary_limits": primary_limits})
    primary_baseline = active_bytes(allocation) if primary_limits is not None else None
    peak_primary_growth = 0
    def phase_growth():
        nonlocal peak_primary_growth
        if primary_limits is not None:
            peak_primary_growth = max(peak_primary_growth, active_bytes(allocation) - primary_baseline)
            return peak_primary_growth
        return accounting.growth()
    root.mkdir(parents=True, exist_ok=True)
    phase_root = root / "guardians" / str(uuid.uuid4())
    phase_root.mkdir(parents=True)
    receipt = {"started_monotonic_ns": began, "cleanup_uncertain": False, "exit": 125}
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if phase == "D":
            accounting = Accounting(allocation / "g-budget.json", began, phase="D")
        budget = accounting.read()
        if (budget.get("inflight") or budget.get("m_guardian") or budget.get("d_guardian")
                or budget.get("closed_guardian") or budget.get("s50_guardian")):
            raise RuntimeError(f"retained shared ownership/inflight blocks {phase} admission")
        phase_deadline = began + primary_limits["phase_ns"] if primary_limits is not None else None
        phase_name = "primary" if primary_limits is not None else "calibration"
        if bounded is not None:
            if phase == "S50":
                from g_pair import approved
            else:
                from m_freeze import approved
            approved(profile, allocation)
            accounting.profile = profile
            try:
                phase_deadline = accounting.begin_calibration(bounded, root, budget,
                    cell_id=None if cell_allowance is None else cell_allowance["cell_id"])
            except ValueError:
                if accounting.calibration is not None:
                    write_budget(accounting.path, budget)
                    accounting.finish(uncertain=False, stopped_ns=time.monotonic_ns())
                raise
        if phase in ("D", "S50") or bounded is not None:
            budget[guardian_key] = {"pid": os.getpid(), "start": process_start(os.getpid()),
                                   "started_monotonic_ns": began, "root": str(root)}
            write_budget(accounting.path, budget)
        remaining_growth = (primary_limits["new_retained_bytes"] if primary_limits is not None
                            else accounting.calibration["remaining_retained_bytes"] if accounting.calibration else None)
        try:
            if primary_limits is not None:
                from g_pair_runtime import assess_primary
                assess_primary({"primary_limits": primary_limits, "profile": profile}, budget, active_bytes(allocation))
            if phase == "D":
                limits = _limits(profile, phase)
            if accounting.total(time.monotonic_ns()) + 20 * 10**9 >= limits[0]:
                raise RuntimeError("shared cleanup reserve unavailable")
            if phase_deadline is not None and time.monotonic_ns() + 20 * 10**9 >= phase_deadline:
                raise RuntimeError(f"{phase_name} phase cleanup reserve unavailable")
            source = Path(__file__).resolve().parents[3]
            sample = resource_sample(source, task)
            load_observations = [observe_load()]
            sample["load"] = load_observations[-1]
            gib = 1024**3
            if (sample["root_free"] < 32 * gib or sample["task_free"] < 250 * gib
                    or sample["memory_available"] < 16 * gib or sample["owned"] >= 200 * gib
                    or active_bytes(allocation) >= limits[1]):
                raise RuntimeError(f"{phase} resource start floor fails")
        except (OSError, ValueError, RuntimeError, KeyError, TypeError) as error:
            if phase == "M" and bounded is None:
                raise
            receipt.update(state="held", reason=f"{type(error).__name__}: {error}",
                           stopped_monotonic_ns=time.monotonic_ns())
            receipt["accounting"] = accounting.finish(uncertain=False,
                stopped_ns=receipt["stopped_monotonic_ns"])
            receipt["charged_ns"] = receipt["accounting"]["charged_ns"]
            receipt["elapsed_ns"] = time.monotonic_ns()-began
            receipt["unreconciled"] = False
            if phase == "M":
                write_guardian(phase_root / "guardian.json", receipt)
            else:
                (phase_root / "guardian.json").write_text(json.dumps(receipt, indent=2)+"\n")
            return receipt
        if phase == "M" and bounded is None:
            budget[guardian_key] = {"pid": os.getpid(), "start": process_start(os.getpid()),
                                   "started_monotonic_ns": began, "root": str(root)}
            write_budget(accounting.path, budget)
        boundary, child = TaskBoundary(task), None
        reader = writer = None
        log_context = BoundedLog(phase_root / "guardian.log") if phase == "M" else (phase_root / "guardian.log").open("a")
        with log_context as log:
            try:
                if diagnostic_memory:
                    from d_diagnostic import memory_snapshot
                    receipt["diagnostic_memory_before"] = memory_snapshot()
                    observed = receipt["diagnostic_memory_before"]
                    if (not observed["complete"] or observed["raw"].get("memory.max") != "4294967296"
                            or observed["raw"].get("memory.swap.max") != "0"):
                        raise RuntimeError("D diagnostic requires observed4GiB/zero-swap memory counters")
                if cancelled[0]:
                    raise InterruptedError("D guardian cancelled before controller entry")
                boundary.create()
                reader, writer = os.pipe()
                selected = command() if callable(command) else command
                argv = [*selected, "--admission-fd", str(reader)] if admission else selected
                child = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT,
                    start_new_session=True, pass_fds=(reader, boundary.procs_fd), preexec_fn=boundary.attach)
                if phase == "M":
                    log.start()
                os.close(reader)
                reader = None
                owner = accounting.read()
                owner[guardian_key].update(controller_pid=child.pid, controller_start=process_start(child.pid))
                write_budget(accounting.path, owner)
                os.write(writer, json.dumps(owner[guardian_key]).encode())
                os.close(writer)
                writer = None
                receipt["controller_pid"] = child.pid
                vm = dict(line.split() for line in Path("/proc/vmstat").read_text().splitlines())
                last_sample, swap_streak = began, 0
                last_swap, sampled_at = int(vm["pswpin"]) + int(vm["pswpout"]), began
                receipt["resources"] = [sample]
                while True:
                    now = time.monotonic_ns()
                    if cancelled[0]:
                        receipt.update(exit=143, cancelled=True, stop="ordinary D SIGTERM cancellation")
                        break
                    if phase == "M" and log.error:
                        receipt.update(exit=125, error=log.error)
                        break
                    code = unreaped_exit(child)
                    if phase_deadline is not None:
                        if now + 20 * 10**9 >= phase_deadline:
                            receipt.update(exit=124, stop=f"{phase_name} phase cleanup reserve")
                            break
                        if phase_growth() >= remaining_growth:
                            receipt.update(exit=124, stop=f"{phase_name} retained growth")
                            break
                    if code is not None:
                        receipt["exit"] = code
                        receipt["child_exit"] = code
                        live = live_owned_pids(child)
                        receipt["boundary_populated_at_controller_exit"] = boundary.populated()
                        if live or receipt["boundary_populated_at_controller_exit"]:
                            receipt["exit"] = code if code else 125
                            receipt["stop"] = "owned work survived controller exit"
                        if not live:
                            child.wait()
                        break
                    current = accounting.read()
                    uncommitted = accounting.uncommitted(current, now)
                    common_wall = max(0, now-began-(current["used_ns"]-accounting.prior_used)-uncommitted)
                    active = current.get("inflight") or {}
                    cell_wall = (now-active["started_monotonic_ns"]
                                 + accounting.prior_cells.get(active["cell"], 0)) if active else 0
                    cumulative_limit, fresh_limit = profile["whole_cell_ns"], None
                    if phase == "D":
                        assert accounting is not None
                        from d_replacements import guardian_limits
                        cumulative_limit, fresh_limit = guardian_limits(
                            root,current,active,accounting.prior_cells,profile["whole_cell_ns"])
                    if (accounting.total(now) + 20 * 10**9 >= limits[0]
                            or cell_wall + 20 * 10**9 >= cumulative_limit
                            or (fresh_limit is not None
                                and now-active["started_monotonic_ns"]+20*10**9 >= fresh_limit)
                            or (common_limit_ns is not None and common_wall + 20 * 10**9 >= common_limit_ns)):
                        receipt.update(exit=124, stop="shared cleanup reserve")
                        break
                    if now - last_sample >= 30 * 10**9:
                        sample = resource_sample(source, task)
                        load_observations.append(observe_load())
                        sample["load"] = load_observations[-1]
                        receipt["resources"].append(sample)
                        vm = dict(line.split() for line in Path("/proc/vmstat").read_text().splitlines())
                        swap = int(vm["pswpin"]) + int(vm["pswpout"])
                        if last_swap is not None:
                            rate = (swap-last_swap)*os.sysconf("SC_PAGE_SIZE")/((now-sampled_at)/10**9)
                            swap_streak = swap_streak + 1 if rate > 64 * 1024**2 else 0
                        last_swap, sampled_at, last_sample = swap, now, now
                        if (sample["root_free"] < 20*gib or sample["task_free"] < 50*gib
                                or sample["memory_available"] < 8*gib or sample["owned"] >= 200*gib
                                or swap_streak >= 3 or active_bytes(allocation) >= limits[1]):
                            receipt.update(exit=124, stop="resource/shared retained floor")
                            break
                    time.sleep(0.05)
            except BaseException as error:
                receipt["error"] = f"{type(error).__name__}: {error}"
                if cancelled[0]:
                    receipt.update(exit=143, cancelled=True, stop="ordinary D SIGTERM cancellation")
            finally:
                for fd in (reader, writer):
                    if fd is not None:
                        os.close(fd)
                try:
                    if child is not None and child.returncode is None:
                        receipt["survivors"] = stop_owned_group(child)
                except BaseException as error:
                    receipt.update(cleanup_uncertain=True, cleanup_error=str(error), exit=125)
                try:
                    boundary.cleanup()
                except BaseException as error:
                    receipt.update(cleanup_uncertain=True, boundary_cleanup_error=str(error), exit=125)
                receipt["stopped_monotonic_ns"] = time.monotonic_ns()
                receipt["boundary"] = boundary.evidence
        if diagnostic_memory:
            receipt["diagnostic_memory_after"] = memory_snapshot()
            receipt["diagnostic_memory_complete"] = (receipt["diagnostic_memory_before"]["complete"]
                                                     and receipt["diagnostic_memory_after"]["complete"])
        if phase == "M" and log.error:
            receipt.update(exit=125, error=log.error)
        try:
            final = resource_sample(source, task)
            load_observations.append(observe_load())
            final["load"] = load_observations[-1]
            receipt.setdefault("resources", []).append(final)
        except Exception as error:
            receipt.update(exit=124, final_resource_error=str(error))
        receipt["load"] = summarize_load(load_observations)
        if phase_deadline is not None:
            if time.monotonic_ns() >= phase_deadline:
                receipt.update(exit=124, stop=f"{phase_name} phase deadline")
            if phase_growth() >= remaining_growth:
                receipt.update(exit=124, stop=f"{phase_name} retained growth")
        receipt["accounting"] = accounting.finish(uncertain=receipt["cleanup_uncertain"],
                                                  stopped_ns=receipt["stopped_monotonic_ns"])
        if phase == "M":
            write_guardian(phase_root / "guardian.json", receipt)
        else:
            (phase_root / "guardian.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def guard(*args, **options):
    """Run the existing guardian, allowing only D ordinary TERM to finish cleanup.

    M/S50 retain their previous signal behavior. A repeated D TERM records the
    same cancellation while cleanup runs; the caller's prior handler is restored.
    """
    if options.get("phase", "M") != "D":
        return _guard(*args, **options)
    cancelled = [False]
    previous = signal.getsignal(signal.SIGTERM)
    def cancel(_number, _frame):
        cancelled[0] = True
    signal.signal(signal.SIGTERM, cancel)
    try:
        return _guard(*args, **options, cancelled=cancelled)
    finally:
        signal.signal(signal.SIGTERM, previous)
