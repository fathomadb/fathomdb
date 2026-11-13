"""Bind G-only table cells to current normal measurement-program commands."""

import json
from pathlib import Path
import subprocess
import time


def execute_stage(cell, manifest, attempt, deadline_ns):
    """Run a selected current G route, retaining its actual program input/output."""
    attempt = Path(attempt)
    attempt.mkdir(parents=True, exist_ok=True)
    config = dict(cell, family=cell["stage"], corpus=manifest["corpus"],
                  queries=manifest["queries"], store=manifest["selected_store"],
                  remaining_cell_ms=max(0, (deadline_ns - time.monotonic_ns()) // 10**6))
    path = attempt / "native-config.json"
    path.write_text(json.dumps(config, indent=2) + "\n")
    with (attempt / "native.log").open("wb") as log:
        child = subprocess.run([*manifest["command"], str(path), str(attempt)],
                               stdout=log, stderr=subprocess.STDOUT, check=False)
    result = json.loads((attempt / "result.json").read_text())
    return {"complete": child.returncode == 0 and result.get("complete") is True,
            "steps": [{"family": config["family"], "exit": child.returncode,
                       "config": str(path), "output": str(attempt), "result": result}]}
