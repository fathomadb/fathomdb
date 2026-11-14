"""M table dispatch primitives; campaign entry/qualification remains separate."""

import json
from pathlib import Path

from campaign import run_campaign


def load_plan():
    """Expand the separately declared46-cell M inventory and unchanged MCP defaults."""
    spec = json.loads(Path(__file__).with_name("m-cells.json").read_text())
    plan = []
    for entry in spec["cells"]:
        cell = dict(spec["defaults"], **entry)
        cell["runtime_status"] = spec["runtime_status"]
        cell["config"] = dict(spec["config"], **{key: cell[key] for key in
                              ("result_format", "search_mode", "use_default_embedder")})
        plan.append(cell)
    return plan


def run_table(root, command, *, plan=None):
    """Dispatch an explicit plan to bounded caller-owned children in fresh attempts."""
    return run_campaign(load_plan() if plan is None else plan, root, command)


def run_budgeted_table(root, command, *, profile, allocation, forecasts, plan=None, prepare=None):
    """Dispatch through a caller-selected shared budget profile and complete forecasts."""
    return run_table(root, command, plan=plan)
