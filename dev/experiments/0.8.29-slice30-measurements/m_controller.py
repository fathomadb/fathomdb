"""M table dispatch primitives; campaign entry/qualification remains separate."""

import json
from pathlib import Path
import time

from campaign import run_campaign
from g_campaign import active_bytes
from m_attempt import CLEANUP_RESERVE_NS, run_attempt
from m_guard import write_budget


COMPONENTS = ("copy", "packaging", "open", "first", "warmups", "timed", "logging",
              "persistence", "polling", "idle", "close", "diagnostics", "failed_attempts")


def _assessment(plan, profile, ledger, forecasts, allocation, *, spent=0, prepared=None):
    time_needed = bytes_needed = 0
    for cell in plan:
        forecast = forecasts[cell["id"]]
        components = forecast["components_ns"]
        if any(type(components[key]) is not int or components[key] < 0 for key in COMPONENTS):
            raise ValueError("component bounds unavailable")
        keys = [key for key in COMPONENTS if not (prepared == cell["id"] and key in ("copy", "packaging"))]
        cost = 2 * sum(components[key] for key in keys)
        retained = forecast["new_retained_bytes"]
        if type(retained) is not int or retained < 0:
            raise ValueError("retained-byte bound unavailable")
        prior = ledger.get("cell_used_ns", {}).get(cell["id"], 0)
        if (type(prior) is not int or prior < 0
                or cost + prior + CLEANUP_RESERVE_NS
                + (spent if cell == plan[0] else 0) > profile["whole_cell_ns"]):
            raise ValueError("whole-cell bound exceeds remaining900s")
        time_needed += cost
        bytes_needed += 2 * retained
    if time_needed + ledger["used_ns"] + spent + (CLEANUP_RESERVE_NS if plan else 0) > profile["total_shared_ns"]:
        raise ValueError("complete remaining table exceeds shared time")
    if bytes_needed + active_bytes(allocation) > profile["total_shared_retained_bytes"]:
        raise ValueError("complete remaining table exceeds retained storage")


def _held(cell, reason):
    return {"id": cell["id"], "cell": cell, "status": "held", "complete": False, "reason": reason}


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


def run_budgeted_table(root, command, *, profile, allocation, forecasts, plan=None, prepare=None,
                       common_reserved_ns=0, common_reserved_bytes=0, register=None):
    """Dispatch through a caller-selected shared budget profile and complete forecasts."""
    began = time.monotonic_ns()
    plan = load_plan() if plan is None else plan
    root = Path(root)
    state_path = root / "campaign-state.json"
    previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
    completed = {cell["id"]: previous[cell["id"]] for cell in plan if cell["id"] in previous
                 and previous[cell["id"]]["status"] == "complete"
                 and previous[cell["id"]].get("cell") == cell}
    remaining = [cell for cell in plan if cell["id"] not in completed]
    allocation = Path(allocation)
    reason = None
    if not isinstance(profile, dict) or profile.get("status") != "APPROVED":
        reason = "shared allocation absent or not approved"
    else:
        try:
            if profile["schema"] != "slice30-m-budget/v1":
                raise ValueError("unrecognized budget profile")
            reservations = (common_reserved_ns, common_reserved_bytes,
                            profile.get("reserved_other_work_ns", 0),
                            profile.get("reserved_other_work_bytes", 0))
            if any(type(value) is not int or value < 0 for value in reservations):
                raise ValueError("bounded common/other-work reservations unavailable")
            profile = dict(profile,
                total_shared_ns=profile["total_shared_ns"]-reservations[0]-reservations[2],
                total_shared_retained_bytes=profile["total_shared_retained_bytes"]-reservations[1]-reservations[3])
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
            _assessment(remaining, profile, ledger, forecasts, allocation)
        except (KeyError, TypeError, ValueError, OSError) as error:
            reason = str(error)
    if reason:
        return [completed.get(cell["id"], _held(cell, reason)) for cell in plan]
    results = dict(completed)
    root.mkdir(parents=True, exist_ok=True)
    for index, cell in enumerate(remaining):
        pending = remaining[index:]
        started = began if index == 0 else time.monotonic_ns()

        def authorize(prepared):
            try:
                _assessment(pending, profile, ledger, forecasts, allocation,
                            spent=time.monotonic_ns() - started,
                            prepared=cell["id"] if prepared else None)
            except (KeyError, TypeError, ValueError, OSError) as error:
                return str(error)
            return None

        reason = authorize(False)
        if reason:
            for held in pending:
                results[held["id"]] = _held(held, reason)
            break
        prior = ledger.setdefault("cell_used_ns", {}).get(cell["id"], 0)
        deadline = min(started + profile["whole_cell_ns"] - prior,
                       started + total_ns - ledger["used_ns"])
        ledger["inflight"] = {"cell": cell["id"], "started_at_ns": time.time_ns(),
                              "started_monotonic_ns": started}
        write_budget(ledger_path, ledger)

        def monitor():
            return ("shared retained storage exhausted"
                    if active_bytes(allocation) > total_bytes else None)

        outcome = run_attempt(cell, root, command, deadline_ns=deadline, prepare=prepare,
                              authorize=authorize, monitor=monitor, register=register)
        elapsed = time.monotonic_ns() - started
        ledger = json.loads(ledger_path.read_text())
        ledger["used_ns"] += elapsed
        ledger["cell_used_ns"][cell["id"]] = prior + elapsed
        clean = not outcome["cleanup_uncertain"] and outcome["live_owned_pids"] == []
        if clean:
            ledger["inflight"] = None
        else:
            ledger["inflight"]["status"] = "cleanup_uncertain"
        write_budget(ledger_path, ledger)
        outcome["charged_ns"] = elapsed
        results[cell["id"]] = outcome
        previous[cell["id"]] = outcome
        state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
        if not clean:
            for held in remaining[index + 1:]:
                results[held["id"]] = _held(held, "prior owned cleanup uncertain")
            break
    return [results[cell["id"]] for cell in plan]
