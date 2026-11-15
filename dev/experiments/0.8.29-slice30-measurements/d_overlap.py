"""Exact-execution D overlap classification; raw intervals are not exclusive costs."""

import json
from pathlib import Path
import sys


_CELL = ("cell_id", "attempt_id", "n", "repeat", "layer")


def _scope(cell):
    return tuple(cell.get(field) for field in _CELL) if isinstance(cell, dict) else None


def _interval(row):
    start, end = row.get("started_ns"), row.get("ended_ns")
    if type(start) is int and type(end) is int and 0 <= start <= end:
        return start, end
    return None


def classify_overlap(case):
    """Require an entire positive pending bracket in one exact call/worker intersection.

    T never borrows C evidence. Missing/lost/error data is incomplete, not zero.
    Every C candidate must match cell, attempt, size, repeat, engine/session and
    the selected natural operation. Bounds are inclusive observed host times;
    no exclusive cost or cross-execution latency correction is inferred.
    """
    cell = case.get("cell", {})
    if cell.get("layer") == "T":
        return {"verdict": "ACTUAL_OVERLAP_UNOBSERVED", "reason": "ordinary timing has no worker collector"}
    incomplete = {"verdict": "INCOMPLETE", "reason": "required exact-execution observation is unavailable"}
    scope = _scope(cell)
    owner = (case.get("engine_id"), case.get("session_id"))
    if (cell.get("layer") != "C" or scope is None or any(value is None for value in scope + owner)
            or case.get("operation_id") is None or case.get("dropped_records") != 0
            or not isinstance(case.get("issues"), list) or case["issues"]
            or not isinstance(case.get("records"), list) or not isinstance(case.get("pending"), list)):
        return incomplete
    queries, workers, pending = [], [], []
    for frame in case["records"]:
        if _scope(frame.get("cell")) != scope:
            continue
        record = frame.get("record", {})
        if (record.get("engine_id"), record.get("session_id")) != owner:
            continue
        if record.get("status") in ("error", "unavailable"):
            return incomplete
        phase = record.get("phase")
        natural = phase in ("api_text", "api_hybrid") and record.get("operation_id") == case["operation_id"]
        if natural or phase == "projection_processing":
            bounds = _interval(record)
            if record.get("status") != "ok" or bounds is None:
                return incomplete
            (queries if natural else workers).append((record, bounds))
    for row in case["pending"]:
        if _scope(row.get("cell")) != scope or (row.get("engine_id"), row.get("session_id")) != owner:
            continue
        bounds = _interval(row)
        count = row.get("pending_count")
        if bounds is None or type(count) is not int or count < 0:
            return incomplete
        pending.append((row, bounds))
    if not queries or not workers or not pending:
        return incomplete
    for query, (query_start, query_end) in queries:
        for worker, (worker_start, worker_end) in workers:
            start, end = max(query_start, worker_start), min(query_end, worker_end)
            if start >= end:
                continue
            for sample, (pending_start, pending_end) in pending:
                if sample["pending_count"] > 0 and start <= pending_start <= pending_end <= end:
                    return {"verdict": "OBSERVED_OVERLAP", "cell": cell,
                            "engine_id": owner[0], "session_id": owner[1],
                            "operation_id": query["operation_id"],
                            "query_span_id": query.get("span_id"), "worker_span_id": worker.get("span_id"),
                            "query_bounds_ns": [query_start, query_end],
                            "worker_bounds_ns": [worker_start, worker_end],
                            "pending_bounds_ns": [pending_start, pending_end],
                            "intersection_bounds_ns": [start, end], "pending_count": sample["pending_count"]}
    return {"verdict": "NO_OBSERVED_OVERLAP", "reason": "no entire positive pending bracket inside an exact call/worker intersection"}


if __name__ == "__main__":
    print(json.dumps(classify_overlap(json.loads(Path(sys.argv[1]).read_text()))))
