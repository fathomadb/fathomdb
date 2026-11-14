"""Fixed G stages, cap accounting and retained attempts; accepted P stays unchanged."""

from itertools import product
import json
from pathlib import Path
import time

from g_attempt import run_g_attempt


def make_g_plan(spec):
    """Expand the reviewed fixed G stages and paired orders without executing models."""
    if (spec["schema"] != "slice30-g/v1" or spec["format"] != 37
            or spec["provider"] != "default" or spec["repeats"] != 2
            or spec["sizes"] != [10000, 100000] or spec["seed_batch"] != 1000
            or spec["adapters"] != ["engine", "sdk"]
            or spec["routes"] != ["text", "hybrid", "embed"]
            or (spec["warmup"], spec["samples"], spec["diagnostic_queries"]) != (500, 1000, 50)
            or spec["write_batch_sizes"] != [1, 10]
            or spec["write_search_ratios"] != [1, 10, 100]
            or spec["stage_order"] != ["G0", "G1", "G-R", "G-O", "G-W"]
            or spec["tiny_cases"] != ["direct_text", "hybrid_no_dependencies", "hybrid_kind_filter",
                                       "source_window", "commit_change", "no_match"]
            or spec["model"] != {"name": "fathomdb-bge-small-en-v1.5",
                                  "revision": "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
                                  "dimension": 384, "head_tokens": 512}
            or spec["limits"] != {"cell_seconds": 900, "campaign_seconds": 14400,
                                   "active_bytes": 16 * 1024**3}):
        raise ValueError("G inventory differs from the approved fixed continuation")
    common = dict(spec["limits"], provider="default", negative_control=False, format=37,
                  seed_batch=1000, observe=False, artifact="ordinary")
    plan = []
    def add(stage, identifier, **fields):
        plan.append(dict(common, stage=stage, id=identifier, name=identifier, **fields))
    add("G0", "g0-smoke", kind="smoke", n=2, adapters=["engine", "sdk"],
        routes=["text", "hybrid", "embed"], warmup=0, samples=16,
        record_first_call=False, fresh_seed=True, repeat=0)
    for size in [10000, 100000]:
        add("G1", f"g1-n{size}", kind="seed", n=size, adapter="sdk", fresh_seed=True,
            repeat=0, lower_subject="g1-n10000" if size == 100000 else None)
    for repeat in [0, 1]:
        adapters = ["engine", "sdk"] if repeat == 0 else ["sdk", "engine"]
        routes = ["text", "hybrid", "embed"] if repeat == 0 else ["embed", "hybrid", "text"]
        for size in ([10000, 100000] if repeat == 0 else [100000, 10000]):
            add("G-R", f"g-r-n{size}-r{repeat}", kind="read", n=size, repeat=repeat,
                adapters=adapters, routes=routes, warmup=500, samples=1000,
                record_first_call=True, fresh_seed=repeat == 1,
                seed_subject=f"g1-n{size}" if repeat == 0 else None)
    add("G-O", "g-o-branches", kind="branches", n=10, repeat=0,
        adapters=["engine", "sdk"], cases=list(spec["tiny_cases"]), fresh_seed=True,
        artifact="diagnostic", observe=True, warmup=0, record_first_call=False)
    for repeat, modes in [(0, [False, True]), (1, [True, False])]:
        for observed in modes:
            add("G-O", f"g-o-natural-r{repeat}-{'on' if observed else 'off'}", kind="natural",
                n=100000, repeat=repeat, artifact="diagnostic", observe=observed,
                adapters=["engine", "sdk"] if repeat == 0 else ["sdk", "engine"],
                routes=["text", "hybrid", "embed"] if repeat == 0 else ["embed", "hybrid", "text"],
                seed_subject="g1-n100000", fresh_seed=False, warmup=0, samples=50,
                record_first_call=False)
    combinations = list(product([1, 10], [1, 10, 100], ["engine", "sdk"],
                                ["text", "hybrid", "embed"]))
    for repeat in [0, 1]:
        for batch, ratio, adapter, route in (combinations if repeat == 0 else reversed(combinations)):
            add("G-W", f"g-w-b{batch}-s{ratio}-{adapter}-{route}-r{repeat}", kind="write",
                n=100000, repeat=repeat, batch_size=batch, searches_per_batch=ratio,
                adapters=[adapter], routes=[route], warmup=500, samples=1000,
                record_first_call=False, fresh_seed=True)
    return plan


def active_bytes(root):
    """Count all retained file lengths, including failed raw/DB/WAL artifacts."""
    return sum(path.stat().st_size for path in Path(root).rglob("*") if path.is_file())



def predict_seed(cell, previous, cell_usage):
    """Forecast fresh preparation at twice observed per-row wall and retained bytes."""
    if cell["n"] < 10000 or cell["id"] == "g1-n10000":
        return {}
    exact = previous.get(f"g1-n{cell['n']}")
    lower = exact if exact and exact.get("status") == "complete" else previous.get("g1-n10000")
    if not lower or lower.get("status") != "complete":
        return {"unavailable": "missing_completed_lower_seed"}
    if not cell.get("fresh_seed"):
        return {} if exact and exact.get("status") == "complete" else {
            "unavailable": "required_seed_not_complete"}
    observed_rows = lower["cell"]["n"]
    observed_ns = max(lower["elapsed_ns"], cell_usage.get(lower["id"], 0))
    return {"seed_subject": lower["id"],
            "seed_ns": (2 * observed_ns * cell["n"] + observed_rows - 1) // observed_rows,
            "seed_bytes": (2 * active_bytes(lower["attempt"]) * cell["n"] + observed_rows - 1) // observed_rows}

def run_g_plan(spec, root, command, prepare=None, allocation_root=None, assess=None, reused=None):
    """Run every fixed G obligation against one shared allocation and absolute cell clock."""
    plan = make_g_plan(spec)
    root = Path(root)
    allocation = Path(allocation_root) if allocation_root is not None else root
    root.mkdir(parents=True, exist_ok=True)
    allocation.mkdir(parents=True, exist_ok=True)
    budget_path = allocation / "g-budget.json"
    budget = (json.loads(budget_path.read_text()) if budget_path.exists() else
              {"schema": "slice30-gd-budget/v1", "used_ns": 0, "inflight": None})
    if (budget.get("schema") != "slice30-gd-budget/v1"
            or type(budget.get("used_ns")) is not int or budget["used_ns"] < 0):
        raise ValueError("invalid shared G/D allocation record")
    cell_usage = budget.setdefault("cell_used_ns", {})
    if not isinstance(cell_usage, dict) or any(type(value) is not int or value < 0 for value in cell_usage.values()):
        raise ValueError("invalid per-cell G allowance record")
    state_path = root / "campaign-state.json"
    previous = ({row["id"]: row for row in json.loads(state_path.read_text())}
                if state_path.exists() else {})
    outcomes = []
    for cell in plan:
        completed = previous.get(cell["id"])
        if completed and completed.get("status") == "complete" and completed.get("cell") == cell:
            outcomes.append(completed)
            (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
            continue
        reason = ("unfinished_shared_attempt" if budget.get("inflight") else
                  "shared_time_exhausted" if budget["used_ns"] >= 14400 * 10**9 else
                  "cell_time_exhausted" if cell_usage.get(cell["id"], 0) >= 900 * 10**9 else
                  "active_storage_exhausted" if active_bytes(allocation) >= 16 * 1024**3 else None)
        if reason:
            outcomes.append({"id": cell["id"], "cell": cell, "status": "not_run_cap",
                             "complete": False, "reason": reason})
        else:
            started = time.monotonic_ns()
            shared_allowance = 14400 * 10**9 - budget["used_ns"]
            cell_allowance = 900 * 10**9 - cell_usage.get(cell["id"], 0)
            deadline = started + min(shared_allowance, cell_allowance)
            budget["inflight"] = {"cell": cell["id"], "started_at_ns": time.time_ns()}
            budget_path.write_text(json.dumps(budget, indent=2) + "\n")
            work_stopped = True
            try:
                assessment = assess(cell, previous, shared_allowance, active_bytes(allocation)) if assess else None
                prediction = predict_seed(cell, previous, cell_usage) if not assessment else {}
                elapsed = time.monotonic_ns() - started
                reason = ("shared_time_exhausted" if elapsed >= shared_allowance else
                          "cell_time_exhausted" if elapsed >= cell_allowance else
                          "active_storage_exhausted" if active_bytes(allocation) >= 16 * 1024**3 else None)
                if reason:
                    outcome = {"id": cell["id"], "cell": cell, "status": "not_run_cap",
                               "complete": False, "reason": reason}
                elif assessment:
                    outcome = dict(assessment, id=cell["id"], cell=cell, complete=False)
                elif (prediction.get("unavailable") is not None
                      or prediction.get("seed_ns", 0) > min(cell_allowance - elapsed, shared_allowance - elapsed)
                      or prediction.get("seed_bytes", 0) + active_bytes(allocation) > 16 * 1024**3):
                    outcome = {"id": cell["id"], "cell": cell, "status": "not_run_cap",
                               "complete": False, "reason": "cap_feasibility", "prediction": prediction}
                else:
                    context = prepare(cell, root) if prepare else None
                    elapsed = time.monotonic_ns() - started
                    reason = ("shared_time_exhausted" if elapsed >= shared_allowance else
                              "cell_time_exhausted" if elapsed >= cell_allowance else
                              "active_storage_exhausted" if active_bytes(allocation) >= 16 * 1024**3 else None)
                    if reason:
                        outcome = {"id": cell["id"], "cell": cell, "status": "not_run_cap",
                                   "complete": False, "reason": reason}
                    else:
                        def monitor(_pid):
                            now_elapsed = time.monotonic_ns() - started
                            if active_bytes(allocation) >= 16 * 1024**3:
                                return "active_storage_exhausted"
                            if now_elapsed >= shared_allowance:
                                return "shared_time_exhausted"
                            if now_elapsed >= cell_allowance:
                                return "cell_time_exhausted"
                            return None
                        work_stopped = False
                        outcome = run_g_attempt(cell, root, command, deadline, context=context, monitor=monitor)
                        work_stopped = not outcome.get("cleanup_uncertain", True)
                outcomes.append(outcome)
                (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
                if outcome.get("status") == "complete" and time.monotonic_ns() >= deadline:
                    outcome.update(status="failed", complete=False, error="whole_cell_time_exhausted")
                    state = {row["id"]: row for row in json.loads(state_path.read_text())}
                    state[cell["id"]] = outcome
                    state_path.write_text(json.dumps(list(state.values()), indent=2) + "\n")
                    Path(outcome["attempt"], "controller-outcome.json").write_text(json.dumps(outcome, indent=2) + "\n")
            finally:
                elapsed = time.monotonic_ns() - started
                budget["used_ns"] += elapsed
                cell_usage[cell["id"]] = cell_usage.get(cell["id"], 0) + elapsed
                if work_stopped:
                    budget["inflight"] = None
                else:
                    budget["inflight"]["status"] = "cleanup_uncertain"
                budget_path.write_text(json.dumps(budget, indent=2) + "\n")
        previous[cell["id"]] = outcomes[-1]
        (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
    return outcomes
