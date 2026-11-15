"""One explicitly selected bounded M calibration, separate from primary admission."""

CELL_ID = "M-R-1-1-warn"
CALIBRATION_ID = "M-CAL:" + CELL_ID


def control(root, owner, manifest=None):
    """Dispatch one admitted selected configuration through existing preparation/payload."""
    return None


def preparation_bound(manifest):
    """Bound the selected closed package and existing bounded retained producers."""
    return None


def selected_cell(cell_id):
    """Return the unchanged selected workload under its separate calibration ID."""
    from m_controller import load_plan
    if cell_id != CELL_ID:
        raise ValueError("only the explicitly selected first calibration is implemented")
    cell = next(row for row in load_plan() if row["id"] == cell_id)
    return dict(cell, id=CALIBRATION_ID, origin_id=cell_id)


def admit(profile, ledger, retained_bytes, limits):
    """Reserve positive remaining calibration allowances on the original shared budget."""
    keys = {"phase_ns", "payload_ns", "new_retained_bytes"}
    if (set(limits) != keys or any(type(limits[k]) is not int or limits[k] <= 0 for k in keys)
            or not 7 * 10**9 < limits["payload_ns"] <= limits["phase_ns"] - 20 * 10**9
            or limits["phase_ns"] > 900 * 10**9):
        raise ValueError("positive bounded calibration allowance required, including cleanup")
    usage = ledger.get("calibration_usage", {}).get(CALIBRATION_ID, {})
    if usage and usage.get("limits") != limits:
        raise ValueError("selected cumulative calibration allowance changed")
    prior_ns, prior_bytes = usage.get("wall_ns", 0), usage.get("new_retained_bytes", 0)
    if any(type(v) is not int or v < 0 for v in (prior_ns, prior_bytes, ledger["used_ns"], retained_bytes)):
        raise ValueError("bounded prior calibration/shared usage required")
    phase_ns = limits["phase_ns"] - prior_ns
    growth = limits["new_retained_bytes"] - prior_bytes
    if phase_ns <= 20 * 10**9 or growth <= 0:
        raise ValueError("remaining calibration allowance cannot fund work and cleanup")
    if (ledger["used_ns"] + 2 * phase_ns + profile["reserved_other_work_ns"] > profile["total_shared_ns"]
            or retained_bytes + 2 * growth + profile["reserved_other_work_bytes"]
            > profile["total_shared_retained_bytes"]):
        raise ValueError("positive twofold calibration headroom unavailable")
    return {"remaining_phase_ns": phase_ns, "remaining_retained_bytes": growth,
            "prior_wall_ns": prior_ns, "prior_retained_bytes": prior_bytes}
