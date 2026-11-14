"""Supervised common-work accounting under one nonrenewable named reserve."""

import json
from pathlib import Path
import time

from g_attempt import run_g_attempt
from g_campaign import active_bytes
from g_common_storage import common_bytes
from g_closed_guard import write_budget

COMMON_BYTES = 1024**3


def run_common(allocation, mode, command):
    """Charge actual common work once; caller owns the allocation lock and guardian.

    A40s cleanup reservation remains outside the child execution deadline. Unknown
    cleanup retains inflight and blocks subsequent normal entry.
    """
    started = time.monotonic_ns()
    allocation = Path(allocation)
    budget_path = allocation / "g-budget.json"
    budget = (json.loads(budget_path.read_text()) if budget_path.exists() else
              {"schema": "slice30-gd-budget/v1", "used_ns": 0, "inflight": None, "cell_used_ns": {}})
    common = budget.setdefault("common_used_ns", {})
    remaining = min(600 * 10**9 - common.get(mode, 0), 14400 * 10**9 - budget["used_ns"])
    reason = ("unfinished_shared_attempt" if budget.get("inflight") else
              "common_time_exhausted" if common.get(mode, 0) >= 600 * 10**9 else
              "shared_time_exhausted" if budget["used_ns"] >= 14400 * 10**9 else
              "common_storage_exhausted" if common_bytes(allocation) >= COMMON_BYTES else
              "shared_storage_exhausted" if active_bytes(allocation) >= 16 * 1024**3 else
              "common_cleanup_reserve" if remaining <= 40 * 10**9 else None)
    if reason:
        return {"status": "not_run_cap", "complete": False, "reason": reason}
    deadline = started + remaining - 40 * 10**9
    budget["inflight"] = {"cell": "g-common-" + mode, "started_at_ns": time.time_ns()}
    write_budget(budget_path, budget)
    outcome = None
    try:
        def monitor(_pid):
            if common_bytes(allocation) >= COMMON_BYTES:
                return "common_storage_exhausted"
            if active_bytes(allocation) >= 16 * 1024**3:
                return "shared_storage_exhausted"
            return None
        outcome = run_g_attempt({"id": "g-common-" + mode}, allocation / "common", command,
                                deadline, monitor=monitor)
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
