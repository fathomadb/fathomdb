"""Supervised common-work accounting under one nonrenewable named reserve."""

import json
from pathlib import Path
import time

from g_attempt import run_g_attempt
from g_campaign import active_bytes
from g_common_storage import common_bytes
from g_closed_guard import write_budget

COMMON_BYTES = 1024**3


def run_common(allocation, mode, command, limits=None, admission_job=None):
    """Charge actual common work once; caller owns the allocation lock and guardian.

    A40s cleanup reservation remains outside the child execution deadline. Unknown
    cleanup retains inflight and blocks subsequent normal entry.
    """
    started = time.monotonic_ns()
    allocation = Path(allocation)
    budget_path = allocation / "g-budget.json"
    budget = (json.loads(budget_path.read_text()) if budget_path.exists() else
              {"schema": "slice30-gd-budget/v1", "used_ns": 0, "inflight": None, "cell_used_ns": {}})
    from g_closed_limits import DEFAULT, validate_limits
    selected_limits = validate_limits(limits or DEFAULT)
    guardian_limits = (budget.get("closed_guardian") or {}).get("limits")
    if (limits is not None and mode != "closed-copy-write-matrix"
            or guardian_limits is not None and selected_limits != guardian_limits):
        raise ValueError("common worker prospective totals differ from its active outer guardian")
    shared_ns = selected_limits["campaign_seconds"]*10**9
    storage_bytes = selected_limits["active_bytes"]
    common = budget.setdefault("common_used_ns", {})
    remaining = min(600 * 10**9 - common.get(mode, 0), shared_ns - budget["used_ns"])
    reason = ("unfinished_shared_attempt" if budget.get("inflight") else
              "common_time_exhausted" if common.get(mode, 0) >= 600 * 10**9 else
              "shared_time_exhausted" if budget["used_ns"] >= shared_ns else
              "common_storage_exhausted" if common_bytes(allocation) >= COMMON_BYTES else
              "shared_storage_exhausted" if active_bytes(allocation) >= storage_bytes else
              "common_cleanup_reserve" if remaining <= 40 * 10**9 else None)
    if reason:
        return {"status": "not_run_cap", "complete": False, "reason": reason}
    deadline = started + remaining - 40 * 10**9
    budget["inflight"] = {"cell": "g-common-" + mode, "started_at_ns": time.time_ns()}
    context = None
    if admission_job is not None:
        from g_admission import controller_admission
        admission = controller_admission(allocation, allocation / "common", {"id": "g-common-"+mode},
                                         deadline, admission_job, mode)
        budget["inflight"]["admission"] = admission
        context = {"common_admission": admission}
    write_budget(budget_path, budget)
    outcome = None
    try:
        def monitor(_pid):
            if common_bytes(allocation) >= COMMON_BYTES:
                return "common_storage_exhausted"
            if active_bytes(allocation) >= storage_bytes:
                return "shared_storage_exhausted"
            return None
        outcome = run_g_attempt({"id": "g-common-" + mode}, allocation / "common", command,
                                deadline, context=context, monitor=monitor)
        return outcome
    finally:
        elapsed = time.monotonic_ns() - started
        budget["used_ns"] += elapsed
        common[mode] = common.get(mode, 0) + elapsed
        if outcome is not None and not outcome.get("cleanup_uncertain", True):
            budget["inflight"] = None
        else:
            budget["inflight"]["status"] = "cleanup_uncertain"
        write_budget(budget_path, budget)
