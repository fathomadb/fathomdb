"""Fresh M attempts; preparation and payload share one absolute owned deadline."""

import json
import os
from pathlib import Path
import select
import signal
import subprocess
import time
import traceback

from g_attempt import live_owned_pids, stop_owned_group, unreaped_exit
from m_record import BoundedLog, CONTROL_MAX_BYTES, OutputLimitError, write_json

CLEANUP_RESERVE_NS = 7 * 10**9


class _Preparation:
    def __init__(self, callback, cell, attempt, deadline_ns, log):
        if len(list(Path("/proc/self/task").iterdir())) != 1:
            raise RuntimeError("preparation requires a single-threaded controller")
        reader, writer = os.pipe()
        self.returncode = None
        self.pid = os.fork()
        if self.pid == 0:
            os.close(reader)
            try:
                os.setsid()
                os.dup2(log.fileno(), 1)
                os.dup2(log.fileno(), 2)
                os.write(writer, b"1")
                os.close(writer)
                callback(cell, attempt, deadline_ns)
            except BaseException:
                traceback.print_exc()
                os._exit(1)
            os._exit(0)
        os.close(writer)
        try:
            remaining = max(0, (deadline_ns - time.monotonic_ns()) / 10**9)
            ready = select.select([reader], [], [], min(2, remaining))[0]
            if not ready or os.read(reader, 1) != b"1":
                os.kill(self.pid, signal.SIGKILL)
                self.wait()
                raise RuntimeError("owned preparation did not establish its session")
        finally:
            os.close(reader)

    def wait(self):
        if self.returncode is None:
            _, status = os.waitpid(self.pid, 0)
            self.returncode = os.waitstatus_to_exitcode(status)
        return self.returncode


def _wait_owned(child, deadline_ns, monitor):
    error, code, uncertain, live = None, None, False, []
    checked = None
    try:
        while True:
            code = unreaped_exit(child)
            now = time.monotonic_ns()
            if checked is None or now - checked >= 10**9 or code is not None:
                error = monitor() if monitor else None
                checked = now
            if now >= deadline_ns:
                error = error or "absolute attempt deadline"
            if error:
                code = 124
                break
            if code is not None:
                live = live_owned_pids(child)
                if live:
                    code, error = 125, "owned work survived leader"
                else:
                    child.wait()
                break
            time.sleep(0.01)
    except BaseException as failure:
        code, error = 125, f"{type(failure).__name__}: {failure}"
    finally:
        if child.returncode is None:
            try:
                live = stop_owned_group(child)
            except BaseException as failure:
                uncertain, live = True, None
                code, error = 125, f"owned cleanup uncertain: {failure}"
    return code, error, uncertain, live


def run_attempt(cell, root, command, *, deadline_ns, prepare=None, authorize=None, monitor=None,
                register=None):
    """Supervise fresh preparation/payload sessions; retain failures and original files."""
    started = time.monotonic_ns()
    work_deadline_ns = deadline_ns - CLEANUP_RESERVE_NS
    cell_root = Path(root) / cell["id"]
    cell_root.mkdir(parents=True, exist_ok=True)
    prior = [int(path.name.split("-")[1]) for path in cell_root.glob("attempt-*") if path.is_dir()]
    attempt = cell_root / f"attempt-{max(prior, default=0) + 1:04}"
    attempt.mkdir()
    bounded = cell.get("family") in ("M0", "M-R", "M-C", "M-L", "M-E")
    config = attempt / "cell.json"
    if bounded:
        write_json(config, cell, limit=CONTROL_MAX_BYTES)
    else:
        config.write_text(json.dumps(cell, indent=2) + "\n")
    outcome = {"id": cell["id"], "cell": cell, "attempt": str(attempt), "status": "failed",
               "complete": False, "exit": None, "cleanup_uncertain": False,
               "live_owned_pids": [], "deadline_ns": deadline_ns,
               "work_deadline_ns": work_deadline_ns, "cleanup_reserve_ns": CLEANUP_RESERVE_NS,
               "sessions": []}
    error = None
    context = BoundedLog(attempt / "child.log") if bounded else (attempt / "child.log").open("a", encoding="utf-8", buffering=1)
    with context as log:
        try:
            if register:
                register(cell, attempt)
            error = authorize(False) if authorize else None
            if not error and time.monotonic_ns() >= work_deadline_ns:
                error = "insufficient work allowance after bounded cleanup reserve"
            if not error and prepare:
                child = _Preparation(prepare, cell, attempt, work_deadline_ns, log)
                if bounded:
                    log.start()
                code, error, uncertain, live = _wait_owned(child, work_deadline_ns,
                    lambda: getattr(log, "error", None) or (monitor() if monitor else None))
                outcome["sessions"].append({"kind": "preparation", "pid": child.pid,
                                             "exit": child.returncode})
                outcome.update(exit=code, cleanup_uncertain=uncertain, live_owned_pids=live)
                if code != 0:
                    error = error or "preparation failed"
            if not error:
                error = authorize(bool(prepare)) if authorize else None
            if not error and time.monotonic_ns() < work_deadline_ns:
                argv = command(cell, attempt, work_deadline_ns) if callable(command) else command
                argv = [*argv, str(config), str(attempt)]
                child = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT,
                                         start_new_session=True)
                if bounded:
                    log.start()
                code, error, uncertain, live = _wait_owned(child, work_deadline_ns,
                    lambda: getattr(log, "error", None) or (monitor() if monitor else None))
                outcome["sessions"].append({"kind": "payload", "pid": child.pid,
                                             "exit": child.returncode})
                outcome.update(exit=code, child_exit=child.returncode, pid=child.pid, pgid=child.pid,
                               command=argv, cleanup_uncertain=uncertain, live_owned_pids=live)
                if code == 0 and error is None:
                    value = json.loads((attempt / "result.json").read_text())
                    outcome["complete"] = value.get("complete") is True
                    outcome["status"] = "complete" if outcome["complete"] else "failed"
            elif not error:
                error = "absolute attempt deadline before payload"
        except BaseException as failure:
            error = f"{type(failure).__name__}: {failure}"
            traceback.print_exc(file=log)
    error = error or getattr(log, "error", None)
    outcome.update(error=error, elapsed_ns=time.monotonic_ns() - started)
    if error:
        outcome.update(complete=False, status="failed")
    try:
        if bounded:
            write_json(attempt / "attempt.json", outcome, limit=CONTROL_MAX_BYTES)
        else:
            (attempt / "attempt.json").write_text(json.dumps(outcome, indent=2) + "\n")
    except OutputLimitError as error:
        outcome.update(complete=False, status="failed", exit=125, error=str(error))
        outcome.pop("command", None)
        write_json(attempt / "attempt.json", outcome, limit=CONTROL_MAX_BYTES)
    return outcome
