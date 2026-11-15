"""Named-phase outer guardian; shared charges survive controller death and owned cleanup."""

import fcntl
import json
import os
from pathlib import Path
import subprocess
import time
import uuid

from g_attempt import live_owned_pids, stop_owned_group, unreaped_exit
from g_boundary import TaskBoundary
from g_campaign import active_bytes
from g_resource import resource_sample
from m_load import observe_load, summarize_load


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
        if phase not in ("M", "D"):
            raise ValueError("unknown shared guardian phase")
        self.phase, self.guardian_key = phase, phase.lower()+"_guardian"
        self.path, self.started_ns = Path(path), started_ns
        prior = self.read()
        self.prior_used = prior["used_ns"]
        self.prior_cells = dict(prior.get("cell_used_ns", {}))

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
        write_budget(self.path, value)
        return {"common_ns": residual, "recovered_cell_ns": recovered,
                "charged_ns": value["used_ns"] - self.prior_used}


def _limits(profile, phase):
    if phase == "D" and any(type(profile.get(key)) is not int or not 0 < profile[key] <= bound
            for key, bound in (("total_shared_ns", 14400*10**9),
                               ("total_shared_retained_bytes", 16*1024**3))):
        raise ValueError("D shared maxima remain14400s/16GiB")
    default = 0 if phase == "M" else None
    reserved = (profile.get("reserved_other_work_ns", default),
                profile.get("reserved_other_work_bytes", default))
    if any(type(value) is not int or value < 0 for value in reserved):
        raise ValueError("bounded other-work reservation unavailable")
    limits = profile["total_shared_ns"]-reserved[0], profile["total_shared_retained_bytes"]-reserved[1]
    if any(type(value) is not int or value <= 0 for value in limits):
        raise RuntimeError(f"shared reserve leaves no positive {phase} allowance")
    return limits


def guard(command, *, allocation, profile, root, task, admission=False, common_limit_ns=None,
          started_ns=None, phase="M"):
    """Own the whole controller phase, private boundary, resource floors and final charge."""
    if phase not in ("M", "D"):
        raise ValueError("unknown shared guardian phase")
    guardian_key = phase.lower()+"_guardian"
    began = time.monotonic_ns() if started_ns is None else started_ns
    allocation, root, task = map(lambda p: Path(p).resolve(), (allocation, root, task))
    if (profile.get("schema") != f"slice30-{phase.lower()}-budget/v1" or profile.get("status") != "APPROVED"
            or profile.get("whole_cell_ns") != 900 * 10**9
            or Path(profile["ledger"]).resolve() != allocation / "g-budget.json"
            or not root.is_relative_to(allocation)):
        raise ValueError(f"approved pinned shared profile and owned {phase} allocation required")
    accounting = Accounting(allocation / "g-budget.json", began) if phase == "M" else None
    limits = _limits(profile, phase) if phase == "M" else None
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
                or budget.get("closed_guardian")):
            raise RuntimeError(f"retained shared ownership/inflight blocks {phase} admission")
        if phase == "D":
            budget[guardian_key] = {"pid": os.getpid(), "start": process_start(os.getpid()),
                                   "started_monotonic_ns": began, "root": str(root)}
            write_budget(accounting.path, budget)
        try:
            if phase == "D":
                limits = _limits(profile, phase)
            if accounting.total(time.monotonic_ns()) + 20 * 10**9 >= limits[0]:
                raise RuntimeError("shared cleanup reserve unavailable")
            source = Path(__file__).resolve().parents[3]
            sample = resource_sample(source, task)
            load_observations = [observe_load()]
            sample["load"] = load_observations[-1]
            if load_observations[-1]["load"] > 2:
                raise RuntimeError(f"{phase} load entry exceeds2: observed {load_observations[-1]['load']}")
            gib = 1024**3
            if (sample["root_free"] < 32 * gib or sample["task_free"] < 250 * gib
                    or sample["memory_available"] < 16 * gib or sample["owned"] >= 200 * gib
                    or active_bytes(allocation) >= limits[1]):
                raise RuntimeError(f"{phase} resource start floor fails")
        except (OSError, ValueError, RuntimeError, KeyError, TypeError) as error:
            if phase == "M":
                raise
            receipt.update(state="held", reason=f"{type(error).__name__}: {error}",
                           stopped_monotonic_ns=time.monotonic_ns())
            receipt["accounting"] = accounting.finish(uncertain=False,
                stopped_ns=receipt["stopped_monotonic_ns"])
            receipt["charged_ns"] = receipt["accounting"]["charged_ns"]
            receipt["elapsed_ns"] = time.monotonic_ns()-began
            receipt["unreconciled"] = False
            (phase_root / "guardian.json").write_text(json.dumps(receipt, indent=2)+"\n")
            return receipt
        if phase == "M":
            budget[guardian_key] = {"pid": os.getpid(), "start": process_start(os.getpid()),
                                   "started_monotonic_ns": began, "root": str(root)}
            write_budget(accounting.path, budget)
        boundary, child = TaskBoundary(task), None
        reader = writer = None
        with (phase_root / "guardian.log").open("a") as log:
            try:
                boundary.create()
                reader, writer = os.pipe()
                selected = command() if callable(command) else command
                argv = [*selected, "--admission-fd", str(reader)] if admission else selected
                child = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT,
                    start_new_session=True, pass_fds=(reader, boundary.procs_fd), preexec_fn=boundary.attach)
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
                    code = unreaped_exit(child)
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
                    if (accounting.total(now) + 20 * 10**9 >= limits[0]
                            or cell_wall + 20 * 10**9 >= profile["whole_cell_ns"]
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
        try:
            final = resource_sample(source, task)
            load_observations.append(observe_load())
            final["load"] = load_observations[-1]
            receipt.setdefault("resources", []).append(final)
        except Exception as error:
            receipt.update(exit=124, final_resource_error=str(error))
        receipt["load"] = summarize_load(load_observations)
        receipt["accounting"] = accounting.finish(uncertain=receipt["cleanup_uncertain"],
                                                  stopped_ns=receipt["stopped_monotonic_ns"])
        (phase_root / "guardian.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt
