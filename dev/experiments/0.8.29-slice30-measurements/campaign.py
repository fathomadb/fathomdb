"""Run Slice30 frozen cells with distinct attempts and retained outcomes."""

import json
from pathlib import Path
import subprocess
import time


def make_plan(cells, repeats):
    """Balance cell/adapter order and assign a distinct repetition identity."""
    plan = []
    for repeat in range(repeats):
        ordered = cells if repeat % 2 == 0 else list(reversed(cells))
        for cell in ordered:
            item = dict(cell, repeat=repeat, id=f"{cell['name']}-r{repeat}")
            if "adapters" in item:
                item["adapters"] = (list(cell["adapters"]) if repeat % 2 == 0
                                    else list(reversed(cell["adapters"])))
            plan.append(item)
    return plan


def run_campaign(plan, root, command, timeout=900, before=None, monitor=None):
    """Execute every cell in an owned fresh attempt directory, retaining failures."""
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
        started = time.monotonic_ns()
        error = None
        with (attempt / "child.log").open("wb") as log:
            child = subprocess.Popen([*command, str(config), str(attempt)],
                                     stdout=log, stderr=subprocess.STDOUT)
            while True:
                remaining = timeout - (time.monotonic_ns() - started) / 1000000000
                try:
                    code = child.wait(timeout=max(0.01, min(30, remaining)))
                    break
                except subprocess.TimeoutExpired:
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
        result_path = attempt / "result.json"
        complete = False
        if code == 0 and error is None:
            if result_path.exists():
                complete = json.loads(result_path.read_text()).get("complete") is True
            else:
                error = "child returned without result.json"
        outcome = {"id": cell["id"], "cell": cell, "attempt": str(attempt), "exit": code,
                   "status": "complete" if complete else "failed", "error": error,
                   "elapsed_ns": time.monotonic_ns() - started, "environment_before": context}
        outcomes.append(outcome)
        (root / "campaign-state.json").write_text(json.dumps(outcomes, indent=2) + "\n")
    return outcomes
