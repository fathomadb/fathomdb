"""D fixed-table driver seam; live supervision and scientific runtime remain gated."""

import time

from d_controller import run_initial_quartet, run_required


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
            result = execute(cell, started + remaining)
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
