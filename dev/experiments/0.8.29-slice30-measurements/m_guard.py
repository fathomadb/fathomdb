"""M-owned outer guardian; shared charges survive controller death and owned cleanup."""

import fcntl
import json
import os
from pathlib import Path
import subprocess
import time

from g_attempt import live_owned_pids, stop_owned_group, unreaped_exit
from g_boundary import TaskBoundary
from g_campaign import active_bytes
from g_resource import resource_sample


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

    def __init__(self, path, started_ns):
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
            raise ValueError("active M interval does not belong to this guardian")
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
        residual = max(0, now - self.started_ns - (value["used_ns"] - self.prior_used))
        value["used_ns"] += residual
        common = value.setdefault("common_used_ns", {})
        common["M"] = common.get("M", 0) + residual
        if uncertain:
            value["m_guardian"]["status"] = "cleanup_uncertain"
        else:
            value["m_guardian"] = None
        write_budget(self.path, value)
        return {"common_ns": residual, "recovered_cell_ns": 0,
                "charged_ns": value["used_ns"] - self.prior_used}


def guard(command, *, allocation, profile, root, task, admission=False):
    """Own the whole controller phase, private boundary, resource floors and final charge."""
    began = time.monotonic_ns()
    allocation, root, task = map(lambda p: Path(p).resolve(), (allocation, root, task))
    if (profile.get("schema") != "slice30-m-budget/v1" or profile.get("status") != "APPROVED"
            or profile.get("whole_cell_ns") != 900 * 10**9
            or Path(profile["ledger"]).resolve() != allocation / "g-budget.json"
            or not root.is_relative_to(allocation)):
        raise ValueError("approved pinned shared profile and owned M allocation required")
    accounting = Accounting(allocation / "g-budget.json", began)
    limits = profile["total_shared_ns"], profile["total_shared_retained_bytes"]
    if any(type(value) is not int or value <= 0 for value in limits):
        raise ValueError("positive approved shared caps required")
    root.mkdir(parents=True, exist_ok=True)
    receipt = {"started_monotonic_ns": began, "cleanup_uncertain": False, "exit": 125}
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        budget = accounting.read()
        if budget.get("inflight") or budget.get("m_guardian") or budget.get("closed_guardian"):
            raise RuntimeError("retained shared ownership/inflight blocks M admission")
        if accounting.total(time.monotonic_ns()) + 10 * 10**9 >= limits[0]:
            raise RuntimeError("shared cleanup reserve unavailable")
        source = Path(__file__).resolve().parents[3]
        sample = resource_sample(source, task)
        gib = 1024**3
        if (sample["root_free"] < 32 * gib or sample["task_free"] < 250 * gib
                or sample["memory_available"] < 16 * gib or sample["owned"] >= 200 * gib):
            raise RuntimeError("M resource start floor fails")
        budget["m_guardian"] = {"pid": os.getpid(), "start": process_start(os.getpid()),
                                "started_monotonic_ns": began, "root": str(root)}
        write_budget(accounting.path, budget)
        boundary, child = TaskBoundary(task), None
        reader = writer = None
        with (root / "guardian.log").open("a") as log:
            try:
                boundary.create()
                reader, writer = os.pipe()
                argv = [*command, "--admission-fd", str(reader)] if admission else command
                child = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT,
                    start_new_session=True, pass_fds=(reader, boundary.procs_fd), preexec_fn=boundary.attach)
                os.close(reader)
                reader = None
                owner = accounting.read()
                owner["m_guardian"].update(controller_pid=child.pid, controller_start=process_start(child.pid))
                write_budget(accounting.path, owner)
                os.write(writer, json.dumps(owner["m_guardian"]).encode())
                os.close(writer)
                writer = None
                receipt["controller_pid"] = child.pid
                last_sample, swap_streak, last_swap, sampled_at = began, 0, None, None
                while True:
                    now = time.monotonic_ns()
                    code = unreaped_exit(child)
                    if code is not None:
                        receipt["exit"] = code
                        if live_owned_pids(child):
                            receipt["exit"] = 125
                        else:
                            child.wait()
                        break
                    if accounting.total(now) + 10 * 10**9 >= limits[0]:
                        receipt.update(exit=124, stop="shared cleanup reserve")
                        break
                    if now - last_sample >= 30 * 10**9:
                        sample = resource_sample(source, task)
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
                    boundary.cleanup()
                except BaseException as error:
                    receipt.update(cleanup_uncertain=True, cleanup_error=str(error), exit=125)
                receipt["stopped_monotonic_ns"] = time.monotonic_ns()
                receipt["boundary"] = boundary.evidence
        receipt["accounting"] = accounting.finish(uncertain=receipt["cleanup_uncertain"],
                                                  stopped_ns=receipt["stopped_monotonic_ns"])
        (root / "guardian.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt
