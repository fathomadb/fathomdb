"""Supervised binding of the actual R1/calibration costs to a reviewed complete G table."""

import json
import os
from pathlib import Path

from g_campaign import active_bytes, make_g_plan
from g_closed_forecast import forecast_closed_write
from g_closed_limits import validate_limits
from g_closed_review import reviewed_costs_match


def read_costs(cell, references):
    """Use the matching qualified ordinary repeat/adapter/route, never an R0 substitution."""
    row = references[f"g-r-n100000-r{cell['repeat']}"]
    actual = json.loads(Path(row["attempt"], "result.json").read_text())
    step = next(item for item in actual["steps"] if item["family"] == "G-R")
    result = step["result"]
    adapter, route = cell["adapters"][0], cell["routes"][0]
    matches = [item for item in result["passes"] if (item["adapter"], item["route"]) == (adapter, route)]
    if len(matches) != 1:
        raise ValueError("matching actual ordinary route wall is unavailable")
    selected = matches[0]
    count = result["sample_count"] + result["warmup_count"] + result["first_call_count"]
    if (any(type(selected.get(key)) is not int or selected[key] <= 0 for key in ("wall_ns", "requests"))
            or type(count) is not int or count <= 0):
        raise ValueError("matching actual ordinary population is unavailable")
    return {"read_repeat": cell["repeat"], "read_adapter": adapter, "read_route": route,
            "route_wall_ns": selected["wall_ns"], "route_requests": selected["requests"],
            "raw_bytes_per_record": (active_bytes(row["attempt"])+count-1)//count}


def prepare_matrix(manifest, calibration):
    """Recompute the recorded full table before dispatch; D/M unknowns remain explicit holds."""
    from g_reuse import collect_reuse
    path = Path(manifest["cost_review_from"])
    review = json.loads(path.read_text())
    if review != manifest.get("cost_review"):
        raise ValueError("reviewed frozen cost prerequisite differs")
    limits = validate_limits(review["limits"])
    if limits != {key: manifest["closed_limits"][key] for key in limits}:
        raise ValueError("reviewed limits differ from frozen prospective limits")
    spec = dict(manifest["spec"], closed_mode="closed-copy-write-matrix",
                _closed_limits=limits, _reviewed_costs=review, _calibration_origin=calibration["origin"],
                _closed_observation_wait_ms=review["observation_wait_ms"])
    plan = make_g_plan(spec)
    if plan != manifest["plan"] or not reviewed_costs_match(spec, plan, manifest["reused"]):
        raise ValueError("reviewed complete G table/R1/calibration/profile binding is unavailable")
    if collect_reuse(manifest, manifest["reuse_completed_from"]) != manifest["reused"]:
        raise ValueError("actual qualified original12 references differ")
    model = review.get("storage_model")
    if model is not None:
        from g_serial_storage import derive_model
        if derive_model(manifest, calibration, manifest["reused"]) != model:
            raise ValueError("reviewed serial storage observations differ")
        spec["_serial_resource_roots"] = {"source": os.environ["S30_SOURCE"], "task": os.environ["S30_TASK"]}
    budget = json.loads(Path(manifest["allocation"], "g-budget.json").read_text())
    retained = active_bytes(manifest["allocation"])
    common_used = budget.get("common_used_ns", {}).get("closed-copy-write-matrix", 0)
    snapshot = review.get("allocation_snapshot") or {}
    keys = ("used_ns", "retained_bytes", "common_used_ns", "remaining_common_ns",
            "remaining_shared_ns", "remaining_retained_bytes")
    if (any(type(snapshot.get(key)) is not int or snapshot[key] < 0 for key in keys)
            or snapshot["used_ns"] > budget["used_ns"] or snapshot["retained_bytes"] > retained
            or snapshot["common_used_ns"] > common_used
            or snapshot["remaining_common_ns"] != max(0, 600*10**9-snapshot["common_used_ns"])
            or snapshot["remaining_shared_ns"] != limits["campaign_seconds"]*10**9-snapshot["used_ns"]
            or snapshot["remaining_retained_bytes"] != limits["active_bytes"]-snapshot["retained_bytes"]):
        raise ValueError("reviewed remaining allowance/retention disposition is unavailable")
    seed = manifest["reused"]["g1-n100000"]
    seed_ns = 2*max(seed["elapsed_ns"], budget.get("cell_used_ns", {}).get(seed["id"], 0))
    legacy = {"components": {"seed_ns": seed_ns}, "allowed": False, "reason": "original_reseed_forecast"}
    predictions = {}
    for cell in plan:
        if cell["stage"] != "G-W":
            continue
        observed = read_costs(cell, manifest["reused"])
        observed["calibration"] = calibration["costs"][cell["adapters"][0]]
        if model is not None:
            observed["serial_storage"] = model
        recorded = review["g_table"][cell["id"]]["prediction"]
        prediction = forecast_closed_write(cell, observed, limits["campaign_seconds"]*10**9-budget["used_ns"],
                                           retained, legacy)
        for key in ("total_ns", "new_bytes", "components", "observed", "legacy_forecast", *(["storage"] if model is not None else [])):
            if prediction.get(key) != recorded.get(key):
                raise ValueError("reviewed actual cost prerequisite changed: " + cell["id"])
        if recorded.get("allowed") is not True:
            prediction = dict(prediction, allowed=False, reason=recorded.get("reason") or "reviewed_cell_held")
        predictions[cell["id"]] = prediction
    admitted = [value for value in predictions.values() if value["allowed"]]
    if model is not None:
        from g_serial_storage import frontier
        disposition = frontier(spec, plan, manifest["reused"], budget, retained)
        if not disposition["allowed"]:
            raise ValueError("reviewed serial G table does not fit: " + disposition["reason"])
    elif (sum(value["total_ns"] for value in admitted)+max(0, 600*10**9-common_used)
            > limits["campaign_seconds"]*10**9-budget["used_ns"]
            or sum(value["new_bytes"] for value in admitted)+retained+1024**3 > limits["active_bytes"]):
        raise ValueError("reviewed admitted G table/common reserve does not fit current shared totals")
    manifest.update(spec=spec, _reviewed_costs=review, _calibration_origin=calibration["origin"],
                    _closed_limits=limits, _closed_predictions=predictions)
