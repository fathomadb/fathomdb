"""G-only retained attempts with absolute deadlines and owned child process groups."""

import json
import os
from pathlib import Path
import signal
import subprocess
import time
import traceback


def unreaped_exit(child):
    """Observe this owned child's exit while leaving its leader PID unreaped."""
    info = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    if info is None:
        return None
    return info.si_status if info.si_code == os.CLD_EXITED else -info.si_status


def wait_unreaped(child, seconds):
    """Wait a bounded interval for exit without releasing the leader's PID pin."""
    until = time.monotonic() + seconds
    while True:
        code = unreaped_exit(child)
        if code is not None or time.monotonic() >= until:
            return code
        time.sleep(0.005)



def live_owned_pids(child):
    """Inspect live members of the owned foreground session while its leader is pinned."""
    if child.returncode is not None:
        raise RuntimeError("cannot inspect a group after its leader pin was reaped")
    unreaped_exit(child)
    leader = Path(f"/proc/{child.pid}/stat").read_text().rsplit(") ", 1)[1].split()
    if (int(leader[2]), int(leader[3])) != (child.pid, child.pid):
        raise RuntimeError("owned leader is not the expected private group/session")
    live = []
    for path in Path("/proc").iterdir():
        if not path.name.isdecimal():
            continue
        try:
            fields = (path / "stat").read_text().rsplit(") ", 1)[1].split()
        except (FileNotFoundError, ProcessLookupError):
            continue
        if (int(fields[2]), int(fields[3])) == (child.pid, child.pid) and fields[0] not in ("Z", "X"):
            live.append(int(path.name))
    return sorted(live)

def stop_owned_group(child):
    """Signal the owned session while its unreaped leader pins the numeric PGID."""
    if child.returncode is not None:
        raise RuntimeError("owned leader was reaped before group cleanup")
    unreaped_exit(child)
    os.killpg(child.pid, signal.SIGTERM)
    wait_unreaped(child, 2)
    os.killpg(child.pid, signal.SIGKILL)
    if wait_unreaped(child, 2) is None:
        raise RuntimeError("owned leader did not stop within bounded group cleanup")
    until = time.monotonic() + 2
    while True:
        live = live_owned_pids(child)
        if not live:
            child.wait()
            return []
        if time.monotonic() >= until:
            raise RuntimeError(f"owned group still has live work after bounded cleanup: {live}")
        time.sleep(0.005)


def run_g_attempt(cell, root, command, deadline_ns, context=None, monitor=None):
    """Retain one fresh G attempt; its config/startup share the parent's deadline."""
    started, started_at = time.monotonic_ns(), time.time()
    root = Path(root)
    cell_root = root / cell["id"]
    cell_root.mkdir(parents=True, exist_ok=True)
    prior = [int(path.name.split("-")[1]) for path in cell_root.glob("attempt-*") if path.is_dir()]
    attempt = cell_root / f"attempt-{max(prior, default=0) + 1:04}"
    attempt.mkdir()
    config = attempt / "cell.json"
    config.write_text(json.dumps(cell, indent=2) + "\n")
    if context is not None:
        (attempt / "context.json").write_text(json.dumps(context, indent=2) + "\n")
    prospective = context.get("prospective_prediction") if isinstance(context, dict) else None
    selected = command(cell, deadline_ns) if callable(command) else command
    argv = [*selected, str(config), str(attempt)]
    error, child, code, complete = None, None, None, False
    cleanup_uncertain = False
    live_owned = []
    forecast_refusal = False
    remaining_at_dispatch = deadline_ns - time.monotonic_ns()
    with (attempt / "child.log").open("w") as log:
        remaining_at_dispatch = deadline_ns - time.monotonic_ns()
        if remaining_at_dispatch <= 0:
            error = "cell timeout before child start"
        elif prospective is not None and (type(prospective.get("total_ns")) is not int
                                          or prospective["total_ns"] < 0
                                          or prospective["total_ns"] > remaining_at_dispatch):
            forecast_refusal = True
            error = "complete forecast exceeds actual remaining allowance or is unavailable"
        else:
            environment = None
            if isinstance(context, dict):
                for key, variable in (("cell_admission", "S30_G_CELL_ADMISSION"),
                                      ("common_admission", "S30_G_COMMON_ADMISSION")):
                    if context.get(key):
                        if environment is None:
                            environment = dict(os.environ)
                        environment[variable] = json.dumps(context[key])
            child = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT, start_new_session=True,
                                     env=environment)
            try:
                checked_at = None
                while True:
                    code = unreaped_exit(child)
                    now = time.monotonic_ns()
                    remaining = (deadline_ns - now) / 10**9
                    if checked_at is None or now - checked_at >= 10**9 or code is not None:
                        error = monitor(child.pid) if monitor else None
                        checked_at = now
                    if remaining <= 0:
                        error = error or "cell timeout"
                    if error:
                        code = 124
                        break
                    if code is not None:
                        live_owned = live_owned_pids(child)
                        if live_owned:
                            error = "owned_work_survived_leader"
                            code = 125
                        else:
                            child.wait()
                        break
                    time.sleep(min(0.01, remaining))
            except BaseException as failure:
                code = 125
                error = f"{type(failure).__name__}: {failure}"
                traceback.print_exc(file=log)
            finally:
                if child.returncode is None:
                    try:
                        live_owned = stop_owned_group(child)
                    except BaseException as failure:
                        cleanup_uncertain = True
                        live_owned = None
                        error = (error or "owned attempt failed") + f"; cleanup {type(failure).__name__}: {failure}"
                        traceback.print_exc(file=log)
            result_path = attempt / "result.json"
            if code == 0 and error is None:
                try:
                    complete = json.loads(result_path.read_text()).get("complete") is True
                except (FileNotFoundError, json.JSONDecodeError):
                    error = "missing or partial result.json"
    outcome = {"id": cell["id"], "cell": cell, "attempt": str(attempt), "exit": code,
               "child_exit": child.returncode if child else None, "cleanup_uncertain": cleanup_uncertain,
               "live_owned_pids": live_owned,
               "status": "not_run_cap" if forecast_refusal else "complete" if complete else "failed", "error": error,
               "elapsed_ns": time.monotonic_ns() - started, "environment_before": context,
               "started_at": started_at, "ended_at": time.time(), "deadline_ns": deadline_ns,
               "pid": child.pid if child else None, "pgid": child.pid if child else None,
               "command": argv}
    if prospective is not None:
        outcome.update(prediction=prospective, remaining_ns_at_dispatch=remaining_at_dispatch,
                       forecast_allowed_at_dispatch=not forecast_refusal)
    if forecast_refusal:
        outcome.update(complete=False, reason="cap_feasibility")
    (attempt / "attempt.json").write_text(json.dumps(outcome, indent=2) + "\n")
    state_path = root / "campaign-state.json"
    previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
    previous[cell["id"]] = outcome
    state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
    return outcome
