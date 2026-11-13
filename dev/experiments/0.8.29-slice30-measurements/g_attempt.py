"""G-only retained attempts with absolute deadlines and owned child process groups."""

import json
import os
from pathlib import Path
import signal
import subprocess
import time


def stop_owned_group(child):
    """Terminate only the new process group created for this owned attempt."""
    for stop_signal in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(child.pid, stop_signal)
        except ProcessLookupError:
            pass
        if stop_signal == signal.SIGTERM:
            try:
                child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                pass
    child.wait()


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
    argv = [*command, str(config), str(attempt)]
    error, child, code, complete = None, None, None, False
    with (attempt / "child.log").open("wb") as log:
        if time.monotonic_ns() >= deadline_ns:
            error = "cell timeout before child start"
        else:
            child = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            while True:
                code = child.poll()
                remaining = (deadline_ns - time.monotonic_ns()) / 10**9
                error = monitor(child.pid) if monitor else None
                if remaining <= 0:
                    error = error or "cell timeout"
                if error:
                    stop_owned_group(child)
                    code = 124
                    break
                if code is not None:
                    break
                try:
                    code = child.wait(timeout=min(1, remaining))
                except subprocess.TimeoutExpired:
                    continue
            result_path = attempt / "result.json"
            if code == 0 and error is None:
                try:
                    complete = json.loads(result_path.read_text()).get("complete") is True
                except (FileNotFoundError, json.JSONDecodeError):
                    error = "missing or partial result.json"
    outcome = {"id": cell["id"], "cell": cell, "attempt": str(attempt), "exit": code,
               "child_exit": child.returncode if child else None,
               "status": "complete" if complete else "failed", "error": error,
               "elapsed_ns": time.monotonic_ns() - started, "environment_before": context,
               "started_at": started_at, "ended_at": time.time(), "deadline_ns": deadline_ns,
               "pid": child.pid if child else None, "pgid": child.pid if child else None,
               "command": argv}
    (attempt / "attempt.json").write_text(json.dumps(outcome, indent=2) + "\n")
    state_path = root / "campaign-state.json"
    previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
    previous[cell["id"]] = outcome
    state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
    return outcome
