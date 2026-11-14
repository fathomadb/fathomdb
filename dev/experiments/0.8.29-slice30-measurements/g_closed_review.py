"""Closed G reviewed-cost prerequisites; other phases are represented, never executed here."""


def reviewed_costs_match(spec, plan, previous):
    """Require the complete reviewed G table and matching R1/calibration/limits.

    This validates the controller protocol after the supervised entry recomputes
    costs from qualified records. It does not authenticate a reviewer or grant
    runtime authority; the session's technical GO remains separate.
    """
    review = spec.get("_reviewed_costs")
    if type(review) is not dict or review.get("schema") != "slice30-closed-write-review/v1":
        return False
    decision = review.get("review") or {}
    r1 = previous.get("g-r-n100000-r1") or {}
    if (review.get("admission_scope") != "G-only" or decision.get("verdict") != "ACCEPT"
            or not decision.get("reviewer") or not decision.get("subject")
            or r1.get("status") not in ("complete", "reused_complete")
            or not r1.get("origin") or review.get("r1") != r1["origin"]
            or not spec.get("_calibration_origin") or review.get("calibration") != spec["_calibration_origin"]
            or review.get("observation_wait_ms") not in (50, 1000)):
        return False
    limits = spec.get("_closed_limits") or spec["limits"]
    if review.get("limits") != {key: limits[key] for key in ("campaign_seconds", "active_bytes")}:
        return False
    others = review.get("other_phases") or []
    if [item.get("phase") for item in others] != ["D", "M"]:
        return False
    unknown = False
    for item in others:
        if item.get("status") == "HELD_COST_UNAVAILABLE":
            unknown = True
            if item.get("forecast_ns") is not None or item.get("new_bytes") is not None:
                return False
        elif (item.get("status") != "FORECAST_AVAILABLE"
              or type(item.get("forecast_ns")) is not int or item["forecast_ns"] < 0
              or type(item.get("new_bytes")) is not int or item["new_bytes"] < 0):
            return False
    if unknown and review.get("combined_feasibility") != "UNKNOWN":
        return False
    table = review.get("g_table")
    writes = [cell for cell in plan if cell["stage"] == "G-W"]
    if type(table) is not dict or list(table) != [cell["id"] for cell in writes]:
        return False
    for cell in writes:
        item = table[cell["id"]]
        prediction = item.get("prediction") or {}
        if item.get("cell") != cell or type(prediction.get("allowed")) is not bool:
            return False
        if prediction["allowed"] and (type(prediction.get("total_ns")) is not int or prediction["total_ns"] < 0
                                      or type(prediction.get("new_bytes")) is not int or prediction["new_bytes"] < 0):
            return False
    return True
