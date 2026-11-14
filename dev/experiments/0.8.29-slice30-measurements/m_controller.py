"""M table dispatch primitives; campaign entry/qualification remains separate."""

import json
from pathlib import Path

from campaign import run_campaign
from g_campaign import active_bytes


COMPONENTS = ("copy", "packaging", "open", "first", "warmups", "timed", "logging",
              "persistence", "polling", "idle", "close", "diagnostics", "failed_attempts")


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
    plan = load_plan() if plan is None else plan
    allocation = Path(allocation)
    reason = None
    if not isinstance(profile, dict) or profile.get("status") != "APPROVED":
        reason = "shared allocation absent or not approved"
    else:
        try:
            if profile["schema"] != "slice30-m-budget/v1":
                raise ValueError("unrecognized budget profile")
            ledger_path = allocation / "g-budget.json"
            if Path(profile["ledger"]).resolve() != ledger_path.resolve():
                raise ValueError("selected shared ledger differs")
            ledger = json.loads(ledger_path.read_text())
            if ledger["schema"] != "slice30-gd-budget/v1" or ledger.get("inflight") is not None:
                raise ValueError("shared ledger is unavailable or inflight")
            total_ns, total_bytes = profile["total_shared_ns"], profile["total_shared_retained_bytes"]
            if (type(total_ns) is not int or total_ns <= 0 or type(total_bytes) is not int
                    or total_bytes <= 0 or profile["whole_cell_ns"] != 900000000000
                    or type(ledger["used_ns"]) is not int or ledger["used_ns"] < 0):
                raise ValueError("invalid shared caps/usage")
            time_needed = bytes_needed = 0
            for cell in plan:
                forecast = forecasts[cell["id"]]
                components = forecast["components_ns"]
                if any(type(components[key]) is not int or components[key] < 0 for key in COMPONENTS):
                    raise ValueError("component bounds unavailable")
                cost = 2 * sum(components[key] for key in COMPONENTS)
                retained = forecast["new_retained_bytes"]
                if type(retained) is not int or retained < 0:
                    raise ValueError("retained-byte bound unavailable")
                prior = ledger.get("cell_used_ns", {}).get(cell["id"], 0)
                if type(prior) is not int or prior < 0 or cost + prior > profile["whole_cell_ns"]:
                    raise ValueError("whole-cell bound exceeds remaining900s")
                time_needed += cost
                bytes_needed += 2 * retained
            if time_needed + ledger["used_ns"] > total_ns:
                raise ValueError("complete remaining table exceeds shared time")
            if bytes_needed + active_bytes(allocation) > total_bytes:
                raise ValueError("complete remaining table exceeds retained storage")
        except (KeyError, TypeError, ValueError, OSError) as error:
            reason = str(error)
    if reason:
        return [{"id": cell["id"], "cell": cell, "status": "held", "complete": False,
                 "reason": reason} for cell in plan]
    return run_table(root, command, plan=plan)
