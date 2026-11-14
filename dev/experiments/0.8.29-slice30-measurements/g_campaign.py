"""Fixed G stages, cap accounting and retained attempts; accepted P stays unchanged."""

from itertools import product
import json
from pathlib import Path
import time

from g_attempt import run_g_attempt

CALIBRATION_CELL_BYTES = 3 * 1024**3
CLOSED_CLEANUP_RESERVE_NS = 40 * 10**9


STAGES = {"branch-controls": "g-o-branches",
          "first-100k-seed-exploratory": "g1-n100000",
          "closed-copy-calibration": None,
          "closed-copy-read-r1": "g-r-n100000-r1",
          "closed-copy-write-matrix": None}


def stage_target(stage):
    """Resolve only the two reviewed staged stops; default keeps the original full plan."""
    if stage is not None and stage not in STAGES:
        raise ValueError("unsupported closed G stage")
    return STAGES.get(stage)


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
    if spec.get("preparation") == "closed-copy-v1":
        for cell in plan:
            if cell["stage"] == "G-W" or cell["id"] == "g-r-n100000-r1":
                cell.update(cell_seconds=1500, preparation="closed-copy-v1",
                            fresh_copy=True, fresh_seed=False)
    return plan


def active_bytes(root):
    """Count all retained file lengths, including failed raw/DB/WAL artifacts."""
    return sum(path.stat().st_size for path in Path(root).rglob("*") if path.is_file())



def completed_record(row):
    """Recognize a completed execution or a validated original-identity reference."""
    return bool(row) and row.get("status") in ("complete", "reused_complete")


def predict_seed(cell, previous, cell_usage):
    """Forecast fresh preparation at twice observed per-row wall and retained bytes."""
    if cell["n"] < 10000 or cell["id"] == "g1-n10000":
        return {}
    exact = previous.get(f"g1-n{cell['n']}")
    lower = exact if completed_record(exact) else previous.get("g1-n10000")
    if not completed_record(lower):
        return {"unavailable": "missing_completed_lower_seed"}
    if not cell.get("fresh_seed"):
        return {} if completed_record(exact) else {
            "unavailable": "required_seed_not_complete"}
    observed_rows = lower["cell"]["n"]
    observed_ns = max(lower["elapsed_ns"], cell_usage.get(lower["id"], 0))
    return {"seed_subject": lower["id"],
            "seed_ns": (2 * observed_ns * cell["n"] + observed_rows - 1) // observed_rows,
            "seed_bytes": (2 * active_bytes(lower["attempt"]) * cell["n"] + observed_rows - 1) // observed_rows}

def run_g_plan(spec, root, command, prepare=None, allocation_root=None, assess=None, reused=None, stage=None, prior_outcome_path=None):
    """Run every fixed G obligation against one shared allocation and absolute cell clock."""
    plan = make_g_plan(spec)
    if spec.get("preparation") == "closed-copy-v1" and spec.get("closed_mode") == "closed-copy-calibration":
        root = Path(root)
        root.mkdir(parents=True, exist_ok=True)
        template = next(cell for cell in plan if cell["id"] == "g-w-b1-s1-engine-text-r0")
        calibration = [dict(template, id=f"g-cal-{adapter}-b{batch}-s{ratio}",
                            name=f"g-cal-{adapter}-b{batch}-s{ratio}", adapters=[adapter],
                            batch_size=batch, searches_per_batch=ratio, calibration=True)
                       for batch, ratio in ((10, 1), (1, 100)) for adapter in ("engine", "sdk")]
        separate = _run_plan(spec, calibration, root / "calibration", command, prepare,
                             allocation_root, assess, None, None, prior_outcome_path,
                             prerequisites=reused)
        (root / "calibration-outcomes.json").write_text(json.dumps(separate, indent=2) + "\n")
        rows = [(reused or {}).get(cell["id"], {"id": cell["id"], "cell": cell,
                 "status": "not_run_stage", "complete": False, "reason": "outside_closed_calibration"})
                for cell in plan]
        (root / "g-outcomes.json").write_text(json.dumps(rows, indent=2) + "\n")
        return rows
    return _run_plan(spec, plan, root, command, prepare, allocation_root, assess,
                     reused, stage, prior_outcome_path)


def _run_plan(spec, plan, root, command, prepare, allocation_root, assess, reused,
              stage, prior_outcome_path, prerequisites=None):
    target = stage_target(stage)
    if spec.get("preparation") == "closed-copy-v1" and spec.get("closed_mode") == "closed-copy-read-r1":
        target = "g-r-n100000-r1"
        stage = spec["closed_mode"]
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
    def save_budget():
        if spec.get("preparation") == "closed-copy-v1":
            from g_closed_guard import write_budget
            write_budget(budget_path, budget)
        else:
            budget_path.write_text(json.dumps(budget, indent=2) + "\n")
    cell_usage = budget.setdefault("cell_used_ns", {})
    if not isinstance(cell_usage, dict) or any(type(value) is not int or value < 0 for value in cell_usage.values()):
        raise ValueError("invalid per-cell G allowance record")
    state_path = root / "campaign-state.json"
    previous = ({row["id"]: row for row in json.loads(state_path.read_text())}
                if state_path.exists() else {})
    previous.update(prerequisites or {})
    if prerequisites:
        state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
    for identifier, reference in (reused or {}).items():
        cell = next((item for item in plan if item["id"] == identifier), None)
        if (cell is None or reference.get("cell") != cell
                or reference.get("status") != "reused_complete" or not reference.get("origin")):
            raise ValueError("invalid original completed reference")
        if not completed_record(previous.get(identifier)):
            previous[identifier] = reference
    if reused:
        state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
    write_mode = (spec.get("preparation") == "closed-copy-v1"
                  and spec.get("closed_mode") == "closed-copy-write-matrix")
    write_reviewed = False
    if write_mode:
        from g_closed_review import reviewed_costs_match
        write_reviewed = reviewed_costs_match(spec, plan, previous)
    outcomes = []
    for cell in plan:
        completed = previous.get(cell["id"])
        if completed_record(completed) and completed.get("cell") == cell:
            outcomes.append(completed)
            (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
            continue
        if write_mode and cell["stage"] != "G-W":
            outcomes.append({"id": cell["id"], "cell": cell, "status": "not_run_stage", "complete": False,
                             "reason": "outside_closed_write_stage"})
            (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
            continue
        stopped = budget.get("closed_calibration_stop") if cell.get("calibration") else None
        if stopped:
            outcomes.append(stopped["outcome"] if stopped["cell"] == cell["id"] else {
                "id": cell["id"], "cell": cell, "status": "not_run_prerequisite", "complete": False,
                "reason": "calibration_stopped", "prior_outcome_path": stopped["path"]})
            (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
            continue
        if target is not None and cell["id"] != target:
            held = {"id": cell["id"], "cell": cell, "status": "not_run_stage",
                    "complete": False, "reason": "outside_closed_stage", "stage": stage}
            if prior_outcome_path:
                held["prior_outcome_path"] = str(prior_outcome_path)
            outcomes.append(held)
            (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
            continue
        if (spec.get("preparation") == "closed-copy-v1"
                and spec.get("closed_mode") == "closed-copy-write-matrix"
                and cell["stage"] == "G-W" and not write_reviewed):
            outcomes.append({"id": cell["id"], "cell": cell, "status": "not_run_prerequisite",
                             "complete": False, "reason": "reviewed_write_costs_required"})
            (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
            continue
        reason = ("unfinished_shared_attempt" if budget.get("inflight") else
                  "shared_time_exhausted" if budget["used_ns"] >= 14400 * 10**9 else
                  "cell_time_exhausted" if cell_usage.get(cell["id"], 0) >= cell["cell_seconds"] * 10**9 else
                  "exploratory_admission_consumed" if stage == "first-100k-seed-exploratory"
                  and budget.get("exploratory_seed100k", {}).get("consumed") else
                  "active_storage_exhausted" if active_bytes(allocation) >= 16 * 1024**3 else None)
        if reason:
            outcomes.append({"id": cell["id"], "cell": cell, "status": "not_run_cap",
                             "complete": False, "reason": reason})
        else:
            started = time.monotonic_ns()
            shared_allowance = 14400 * 10**9 - budget["used_ns"]
            cell_allowance = cell["cell_seconds"] * 10**9 - cell_usage.get(cell["id"], 0)
            deadline = started + min(shared_allowance, cell_allowance)
            budget["inflight"] = {"cell": cell["id"], "started_at_ns": time.time_ns()}
            if spec.get("preparation") == "closed-copy-v1":
                budget["inflight"]["started_monotonic_ns"] = started
            save_budget()
            work_stopped = True
            try:
                assessment = assess(cell, previous, shared_allowance, active_bytes(allocation)) if assess else None
                prediction = predict_seed(cell, previous, cell_usage) if not assessment else {}
                exploratory = stage == "first-100k-seed-exploratory" and cell["id"] == "g1-n100000"
                if exploratory and "seed_ns" in prediction:
                    prediction = dict(prediction, linear_seed_ns=(prediction["seed_ns"] + 1) // 2,
                                      admission_policy="one_shot_exploratory_100k")
                forecast_ns = prediction.get("linear_seed_ns" if exploratory else "seed_ns", 0)
                elapsed = time.monotonic_ns() - started
                reason = ("shared_time_exhausted" if elapsed >= shared_allowance else
                          "cell_time_exhausted" if elapsed >= cell_allowance else
                          "active_storage_exhausted" if active_bytes(allocation) >= 16 * 1024**3 else None)
                if exploratory and "seed_ns" in prediction:
                    prediction["conservative_time_feasible"] = prediction["seed_ns"] <= min(
                        cell_allowance - elapsed, shared_allowance - elapsed)
                    prediction["total_ns"] = forecast_ns
                if reason:
                    outcome = {"id": cell["id"], "cell": cell, "status": "not_run_cap",
                               "complete": False, "reason": reason}
                elif assessment:
                    outcome = dict(assessment, id=cell["id"], cell=cell, complete=False)
                elif (prediction.get("unavailable") is not None
                      or forecast_ns > min(cell_allowance - elapsed, shared_allowance - elapsed)
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
                    elif exploratory and forecast_ns > min(cell_allowance - elapsed, shared_allowance - elapsed):
                        outcome = {"id": cell["id"], "cell": cell, "status": "not_run_cap",
                                   "complete": False, "reason": "cap_feasibility", "prediction": prediction}
                    else:
                        if exploratory:
                            prediction["conservative_time_feasible"] = prediction["seed_ns"] <= min(
                                cell_allowance - elapsed, shared_allowance - elapsed)
                            context = dict(context or {}, prospective_prediction=prediction)
                            # Consume the admission durably before any child can start, including a new-root retry.
                            budget["exploratory_seed100k"] = {"consumed": True, "cell": cell["id"],
                                "root": str(root), "consumed_at_ns": time.time_ns(), "prediction": prediction}
                            save_budget()
                        peak_retained = peak_store = 0
                        def observe_growth():
                            nonlocal peak_retained, peak_store
                            directory = root / cell["id"]
                            peak_retained = max(peak_retained, active_bytes(directory))
                            physical = sum(path.stat().st_size for path in directory.rglob("store.sqlite*")
                                           if path.is_file() and path.name in ("store.sqlite", "store.sqlite-wal", "store.sqlite-shm"))
                            peak_store = max(peak_store, physical)
                        def monitor(_pid):
                            if spec.get("preparation") == "closed-copy-v1":
                                observe_growth()
                            now_elapsed = time.monotonic_ns() - started
                            if cell.get("calibration") and active_bytes(root / cell["id"]) >= CALIBRATION_CELL_BYTES:
                                return "calibration_cell_storage_exhausted"
                            if active_bytes(allocation) >= 16 * 1024**3:
                                return "active_storage_exhausted"
                            if now_elapsed >= shared_allowance:
                                return "shared_time_exhausted"
                            if now_elapsed >= cell_allowance:
                                return "cell_time_exhausted"
                            return None
                        work_stopped = False
                        outcome = run_g_attempt(cell, root, command, deadline, context=context, monitor=monitor)
                        if spec.get("preparation") == "closed-copy-v1":
                            observe_growth()
                            outcome.update(peak_retained_bytes=peak_retained, peak_store_bytes=peak_store,
                                           peak_scope="observed at controller polls/exit; unsampled peaks are not claimed")
                            Path(outcome["attempt"], "controller-outcome.json").write_text(json.dumps(outcome, indent=2) + "\n")
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
                save_budget()
        if cell.get("calibration") and not completed_record(outcomes[-1]):
            budget.setdefault("closed_calibration_stop", {"cell": cell["id"], "outcome": outcomes[-1],
                "path": str(root / "g-outcomes.json")})
            save_budget()
        previous[cell["id"]] = outcomes[-1]
        (root / "g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
    return outcomes
