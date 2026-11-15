"""D-owned subprocess binding; no campaign admission or allocation reset."""

import json
from pathlib import Path
import subprocess
import time
import traceback

from g_attempt import live_owned_pids, stop_owned_group, unreaped_exit


CLEANUP_RESERVE_NS = 6*10**9


def _sample(attempt, stream):
    started = time.monotonic_ns()
    sizes = {}
    for name in ("store.sqlite", "store.sqlite-wal"):
        try:
            sizes[name] = (attempt/name).stat().st_size
        except FileNotFoundError:
            sizes[name] = None
    retained = sum(path.stat().st_size for path in attempt.rglob("*") if path.is_file())
    sample = {"started_ns": started, "ended_ns": time.monotonic_ns(),
              "db_bytes": sizes["store.sqlite"], "wal_exists": sizes["store.sqlite-wal"] is not None,
              "wal_bytes": sizes["store.sqlite-wal"], "retained_payload_bytes": retained}
    stream.write(json.dumps(sample)+"\n")
    stream.flush()
    return retained


def run_owned_cell(cell, root, command, deadline_ns, *, monitor=None):
    """Retain one D child and its native result under the caller's absolute deadline.

    The caller owns admission/locking. Return actual cleanup, elapsed and retained
    cost facts; unavailable results stay unavailable. This seam grants no runtime
    permission and cannot qualify a native report by child exit alone. Reserve
    the existing three2s process-group cleanup bounds inside the outer deadline.
    """
    started = time.monotonic_ns()
    cell_root = Path(root)/cell["id"]
    cell_root.mkdir(parents=True, exist_ok=True)
    prior = [int(path.name.split("-")[1]) for path in cell_root.glob("attempt-*") if path.is_dir()]
    attempt = cell_root/f"attempt-{max(prior, default=0)+1:04}"
    attempt.mkdir()
    child_deadline = deadline_ns-CLEANUP_RESERVE_NS
    config = attempt/"cell.json"
    config.write_text(json.dumps(dict(cell, attempt_id=attempt.name,
        remaining_cell_ms=max(0, (child_deadline-time.monotonic_ns())//10**6)), indent=2)+"\n")
    argv = [*command, str(config), str(attempt)]
    child, code, error, native, peak = None, None, None, None, None
    live, uncertain = [], False
    with (attempt/"child.log").open("w") as log, (attempt/"io.jsonl").open("w") as samples:
        try:
            if time.monotonic_ns() >= child_deadline:
                error = "cell timeout before child start or cleanup allowance unavailable"
            else:
                child = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT,
                                         start_new_session=True)
                checked_at = None
                while True:
                    code = unreaped_exit(child)
                    now = time.monotonic_ns()
                    if checked_at is None or now-checked_at >= 10**9 or code is not None:
                        peak = max(peak or 0, _sample(attempt, samples))
                        if monitor:
                            error = monitor(child.pid)
                        checked_at = now
                    if time.monotonic_ns() >= child_deadline:
                        error = error or "cell timeout"
                    if error:
                        code = 124
                        break
                    if code is not None:
                        live = live_owned_pids(child)
                        if live:
                            error, code = "owned_work_survived_leader", 125
                        else:
                            child.wait()
                        break
                    time.sleep(min(0.01, max(0, (child_deadline-time.monotonic_ns())/10**9)))
        except BaseException as failure:
            error, code, peak = f"{type(failure).__name__}: {failure}", 125, None
            traceback.print_exc(file=log)
        finally:
            if child is not None and child.returncode is None:
                try:
                    live = stop_owned_group(child)
                except BaseException as failure:
                    uncertain, live = True, None
                    error = (error or "owned D attempt failed")+f"; cleanup {type(failure).__name__}: {failure}"
                    traceback.print_exc(file=log)
        try:
            native = json.loads((attempt/"result.json").read_text())
        except (OSError, ValueError) as failure:
            if code == 0 and error is None:
                error = f"native result unavailable: {type(failure).__name__}: {failure}"
        try:
            retained = _sample(attempt, samples)
            if peak is not None:
                peak = max(peak, retained)
            elif child is None and error is not None:
                peak = retained
        except OSError as failure:
            peak = None
            error = (error or "owned D attempt failed")+f"; retained cost {type(failure).__name__}: {failure}"
    complete = (code == 0 and error is None and not uncertain and live == []
                and isinstance(native, dict) and native.get("complete") is True
                and time.monotonic_ns() < deadline_ns)
    if not complete and error is None:
        error = "native result incomplete or outer cell deadline exhausted"
    outcome = {"id": cell["id"], "cell": cell, "attempt_id": attempt.name, "attempt": str(attempt),
               "status": "complete" if complete else "failed", "error": error, "native": native,
               "exit": code, "child_exit": child.returncode if child is not None else None,
               "pid": child.pid if child is not None else None,
               "pgid": child.pid if child is not None else None,
               "cleanup_uncertain": uncertain, "live_owned_pids": live,
               "deadline_ns": deadline_ns, "child_deadline_ns": child_deadline,
               "elapsed_ns": time.monotonic_ns()-started, "peak_bytes": peak,
               "cost_scope": "observed attempt payload including DB/WAL/raw/config/log/result; excludes this outcome JSON",
               "command": argv}
    (attempt/"attempt.json").write_text(json.dumps(outcome, indent=2)+"\n")
    return outcome
