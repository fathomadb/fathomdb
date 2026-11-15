"""D fixed-table driver seam; live supervision and scientific runtime remain gated."""

import time

from d_controller import run_initial_quartet, run_required
from d_report import bind_execution


def _qualify_native(cell, result):
    """Bind a successful selected executor result before publishing lower evidence.

    The executor retains native JSON and supplies any streamed C observations
    separately. Missing C streams/native result/cost stays incomplete; failed
    prefixes return unchanged. This does not launch or authorize an executor.
    """
    row = dict(result)
    if row.get("status") != "complete":
        return row
    native, attempt = row.get("native"), row.get("attempt_id")
    if not isinstance(native, dict) or not isinstance(attempt, str) or not attempt:
        row.update(status="incomplete", error=row.get("error") or "current native execution report unavailable",
                   execution_report=None, physical_count=None, closed=None, error_count=None,
                   observations_complete=None, actual_overlap=None)
        return row
    identity = {"cell_id": cell["id"], "attempt_id": attempt, "n": cell["n"],
                "mode": cell["mode"], "layer": cell["layer"], "repeat": cell["repeat"]}
    observation = row.pop("observation", native.get("observation"))
    report = bind_execution(identity, native, observation)
    calls = native.get("calls")
    errors = (sum(call["ok"] is False for call in calls) if isinstance(calls, list)
              and all(type(call.get("ok")) is bool for call in calls) else None)
    physical = (native.get("final") or {}).get("physical_rows")
    peak = row.get("peak_bytes")
    cost_available = type(peak) is int and peak > 0
    row.update(execution_report=report, physical_count=physical if type(physical) is int else None,
               closed=native.get("close_succeeded"), error_count=errors,
               observations_complete=report["observations_complete"], actual_overlap=report["actual_overlap"],
               dropped_records=observation.get("dropped_records") if isinstance(observation, dict) else None)
    if not report["complete"] or not cost_available:
        row["status"] = "incomplete"
        row["error"] = row.get("error") or (
            "observed owned storage cost unavailable" if not cost_available
            else "native execution report is incomplete: " + ", ".join(report["issues"]))
    return row


def run_accounted_stage(spec, records, budget, execute, *, initial=True, clock=time.monotonic_ns):
    """Return fixed-table outcomes and update caller-owned actual wall charges.

    The caller owns the allocation lock and supplies a supervised executor that
    includes preparation and cleanup before returning. Its absolute deadline is
    bounded by the remaining shared/cell allowance. Nested child wall is retained
    separately; only observed outer time is charged. Unresolved cleanup keeps
    ownership blocked. This callable seam neither acquires a live allocation nor
    grants runtime admission; the initial-quartet stop remains the default.
    """
    through = clock()

    def charge(now, cell_id=None):
        nonlocal through
        elapsed = now - through
        if elapsed < 0:
            raise RuntimeError("D monotonic clock moved backwards")
        budget["used_ns"] += elapsed
        if elapsed:
            if cell_id is None:
                common = budget.setdefault("common_used_ns", {})
                common["D"] = common.get("D", 0) + elapsed
            else:
                cells = budget.setdefault("cell_used_ns", {})
                cells[cell_id] = cells.get(cell_id, 0) + elapsed
        through = now

    def prepare(cell):
        started = clock()
        charge(started)
        if budget.get("inflight"):
            return {"status": "not_run_ownership", "reason": "owned attempt cleanup is unresolved"}
        spent = budget.get("cell_used_ns", {}).get(cell["id"], 0)
        remaining = min(cell["cell_ns"] - spent, budget["shared_ns"] - budget["used_ns"])
        if remaining <= 0 or budget["retained_bytes"] >= budget["storage_limit"]:
            return {"status": "not_run_cap_feasibility", "reason": "remaining shared/cell allowance exhausted"}
        budget["inflight"] = {"cell": cell["id"], "started_monotonic_ns": started}
        result = None
        try:
            result = _qualify_native(cell, execute(cell, started + remaining))
        finally:
            ended = clock()
            charge(ended, cell["id"])
            quiescent = (isinstance(result, dict) and result.get("cleanup_uncertain") is False
                         and result.get("live_owned_pids") == [])
            if quiescent:
                budget["inflight"] = None
            else:
                budget["inflight"].update(status="cleanup_uncertain",
                                          accounted_through_monotonic_ns=ended)
        row = dict(result, elapsed_ns=ended - started, child_elapsed_ns=result.get("elapsed_ns"))
        if not quiescent:
            row["status"] = "failed"
        records[cell["id"]] = row
        return row

    run = run_initial_quartet if initial else run_required
    try:
        return run(spec, records, budget, prepare)
    finally:
        charge(clock())
