"""Run Slice30 frozen cells with distinct attempts and retained outcomes."""

import json
import os
from pathlib import Path
import subprocess
import time
from typing import Any


def make_plan(cells, repeats):
    """Balance cell/adapter order and assign a distinct repetition identity."""
    plan = []
    for repeat in range(repeats):
        ordered = cells if repeat % 2 == 0 else list(reversed(cells))
        for cell in ordered:
            item: dict[str, Any] = dict(cell, repeat=repeat, id=f"{cell['name']}-r{repeat}")
            if "adapters" in item:
                item["adapters"] = (list(cell["adapters"]) if repeat % 2 == 0
                                    else list(reversed(cell["adapters"])))
            plan.append(item)
    return plan


def run_campaign(plan, root, command, timeout=900, before=None, monitor=None, completion=None,
                 poll_interval: float = 30, observe_start=False, execute=None):
    """Run fresh attempts; optional completion/execute callbacks adapt native protocols and supervision."""
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True)
    state_path = root / "campaign-state.json"
    previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
    outcomes = []
    for cell in plan:
        completed = previous.get(cell["id"])
        if completed and completed["status"] == "complete" and completed.get("cell") == cell:
            outcomes.append(completed)
            continue
        context = before() if before else None
        cell_root = root / cell["id"]
        cell_root.mkdir(exist_ok=True)
        prior = [int(p.name.split("-")[1]) for p in cell_root.glob("attempt-*") if p.is_dir()]
        attempt = cell_root / f"attempt-{max(prior, default=0) + 1:04}"
        attempt.mkdir()
        config = attempt / "cell.json"
        config.write_text(json.dumps(cell, indent=2) + "\n")
        started_at = time.time()
        started = time.monotonic_ns()
        error = None
        execution = None
        if execute is not None:
            execution = execute(cell, command, config, attempt, monitor)
            code, error = execution["exit"], execution["error"]
            pid, group = execution["pid"], execution["pgid"]
        else:
            with (attempt / "child.log").open("wb") as log:
                child = subprocess.Popen([*command, str(config), str(attempt)],
                                         stdout=log, stderr=subprocess.STDOUT)
                group = os.getpgid(child.pid)
                initial_observation = observe_start
                while True:
                    remaining = timeout - (time.monotonic_ns() - started) / 1000000000
                    if not initial_observation:
                        try:
                            code = child.wait(timeout=max(0.01, min(poll_interval, remaining)))
                            break
                        except subprocess.TimeoutExpired:
                            pass
                    initial_observation = False
                    error = "cell timeout" if remaining <= 0 else (monitor(child.pid) if monitor else None)
                    if error:
                        child.terminate()
                        try:
                            child.wait(timeout=15)
                        except subprocess.TimeoutExpired:
                            child.kill()
                            child.wait()
                        code = 124
                        break
                if monitor:
                    error = error or monitor(child.pid)
            pid = child.pid
        result_path = attempt / "result.json"
        complete = False
        if code == 0 and error is None:
            if completion is not None:
                complete = completion(cell, attempt) is True
                if not complete:
                    error = "child returned without verified native completion"
            elif result_path.exists():
                complete = json.loads(result_path.read_text()).get("complete") is True
            else:
                error = "child returned without result.json"
        outcome = {"id": cell["id"], "cell": cell, "attempt": str(attempt), "exit": code,
                   "status": "complete" if complete else "failed", "error": error,
                   "elapsed_ns": time.monotonic_ns() - started, "environment_before": context,
                   "started_at": started_at, "ended_at": time.time(), "pid": pid,
                   "pgid": group, "command": [*command, str(config), str(attempt)]}
        if execution is not None:
            outcome["cleanup"] = execution["cleanup"]
            outcome["child_exit"] = execution["child_exit"]
            if execution.get("controller_signal") is not None:
                outcome["controller_signal"] = execution["controller_signal"]
            if execution.get("resource_samples") is not None:
                outcome["resource_samples"] = execution["resource_samples"]
            if execution["pid"] is None:
                outcome["status"] = "not_run_resource"
                outcome["reason"] = execution["error"]
        outcomes.append(outcome)
        previous[cell["id"]] = outcome
        state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
    return outcomes
