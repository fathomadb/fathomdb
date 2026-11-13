"""Frozen G production runner, using current bounded driver and attempt contracts."""

import json
from pathlib import Path

from g_driver import execute_stage


def execute_cell(cell, manifest, attempt, deadline_ns):
    """Execute the fixed cell and retain its complete production qualification result."""
    result = execute_stage(cell, manifest, attempt, deadline_ns)
    (Path(attempt) / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result
