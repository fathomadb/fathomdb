"""Bind G-only table cells to current normal measurement-program commands."""

import json
from pathlib import Path
import subprocess
import time


def run_program_step(command, config, output, deadline_ns):
    """Retain one normal-program step under the same absolute parent cell deadline."""
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    path = output / "native-config.json"
    config = dict(config, remaining_cell_ms=max(0, (deadline_ns - time.monotonic_ns()) // 10**6))
    path.write_text(json.dumps(config, indent=2) + "\n")
    step = {"family": config["family"], "config": str(path), "output": str(output),
            "complete": False, "exit": None, "result": None}
    remaining = (deadline_ns - time.monotonic_ns()) / 10**9
    if config["remaining_cell_ms"] == 0 or remaining <= 0:
        return dict(step, status="not_run_cap", reason="cell_time_exhausted")
    with (output / "native.log").open("wb") as log:
        try:
            child = subprocess.run([*command, str(path), str(output)], stdout=log,
                                   stderr=subprocess.STDOUT, check=False, timeout=remaining)
        except subprocess.TimeoutExpired:
            return dict(step, status="failed", reason="cell_time_exhausted")
    step["exit"] = child.returncode
    try:
        step["result"] = json.loads((output / "result.json").read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return dict(step, status="failed", reason="missing_or_partial_result")
    step["complete"] = (child.returncode == 0 and step["result"].get("complete") is True
                        and time.monotonic_ns() < deadline_ns)
    return dict(step, status="complete" if step["complete"] else "failed")


def execute_stage(cell, manifest, attempt, deadline_ns):
    """Bind a current G driver; G-W always seeds its own matching fresh subject first."""
    attempt = Path(attempt)
    attempt.mkdir(parents=True, exist_ok=True)
    common = dict(cell, corpus=manifest["corpus"], queries=manifest["queries"])
    steps = []
    if cell["stage"] == "G-W":
        seed_output = attempt / "seed"
        seed_config = dict(common, family="G-seed", adapter=cell["adapters"][0])
        seed = run_program_step(manifest["command"], seed_config, seed_output, deadline_ns)
        steps.append(seed)
        if not seed["complete"]:
            return {"complete": False, "steps": steps}
        config = dict(common, family="G-W", store=str(seed_output / "store.sqlite"))
        output = attempt / "write"
    else:
        config = dict(common, family=cell["stage"], store=manifest["selected_store"])
        output = attempt
    steps.append(run_program_step(manifest["command"], config, output, deadline_ns))
    return {"complete": all(step["complete"] for step in steps), "steps": steps}
