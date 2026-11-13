"""Explain worker/query/write overlap on one attempt-local monotonic clock."""

import json
from pathlib import Path
import sys


def analyze(queries, writes, workers, readiness, enabled=True, dropped=0, attribution_complete=True):
    """Classify one attempt's aligned spans; witnesses index the supplied records.

    Native callers carry missing-record knowledge in attribution_complete.
    Invalid worker clocks and loss prevent complete attribution even when an
    independent valid witness survives. Workload success is accounted separately.
    """
    if not enabled:
        return {"status": "WORKER_ATTRIBUTION_UNAVAILABLE", "witnesses": None,
                "partial_intersections": None, "outside_readiness": None,
                "unavailable_workers": None}
    unavailable = [index for index, worker in enumerate(workers)
                   if _interval(worker) is None or worker.get("clock_issue")
                   or worker.get("unavailable")]
    valid_workers = [(index, worker) for index, worker in enumerate(workers)
                     if index not in unavailable]
    witnesses = []
    used = set()
    partial = 0
    for qi, query in enumerate(queries):
        q0, q1 = query["start_ns"], query["end_ns"]
        for wi, write in enumerate(writes):
            w0, w1 = write["start_ns"], write["end_ns"]
            if max(q0, w0) >= min(q1, w1):
                continue
            for ji, worker in valid_workers:
                j0, j1 = worker["start_ns"], worker["end_ns"]
                if max(q0, j0) >= min(q1, j1):
                    continue
                lower, upper = max(q0, w0, j0), min(q1, w1, j1)
                if lower >= upper:
                    partial += 1
                    continue
                for pi, ready in enumerate(readiness):
                    interval = _interval(ready)
                    pending = ready.get("pending_count")
                    if (ready.get("ok") is True and interval is not None
                            and type(pending) is int and pending > 0
                            and lower < interval[0] <= interval[1] < upper):
                        used.add(pi)
                        witnesses.append({"query": qi, "write": wi, "worker": ji,
                                          "readiness": pi, "start_ns": lower, "end_ns": upper})
    complete = attribution_complete and not dropped and not unavailable and bool(workers)
    status = ("INCOMPLETE_ATTRIBUTION" if not complete else
              "OBSERVED_OVERLAP" if witnesses else "INCOMPLETE_OVERLAP")
    return {"status": status, "witnesses": witnesses, "partial_intersections": partial,
            "outside_readiness": [index for index in range(len(readiness)) if index not in used],
            "unavailable_workers": unavailable}


def _interval(row):
    start, end = row.get("start_ns"), row.get("end_ns")
    if type(start) is int and type(end) is int and start <= end:
        return start, end
    return None


def main():
    """Read one JSON analysis request and emit its analysis report."""
    request = json.load(sys.stdin)
    if "native_output" in request:
        root = Path(request["native_output"])
        queries = [json.loads(line) for line in (root / "queries.jsonl").read_text().splitlines()]
        writes = json.loads((root / "writes.json").read_text())
        ready = json.loads((root / "readiness.json").read_text())
        workers = json.loads((root / "worker-observer.json").read_text())
        result = {"status": "NOT_EVALUATED", "timed_queries": sum(row["phase"] == "timed" for row in queries),
                  "write_calls": len(writes), "readiness_calls": len(ready),
                  "worker_records": len(workers.get("records") or []) if workers["available"] else None}
        result.update(analyze(
            queries=[row for row in queries if row["phase"] == "timed"], writes=writes,
            workers=workers["records"] or [], readiness=ready,
            enabled=workers["available"] is True,
            dropped=workers["dropped"] if workers["available"] else 0,
            attribution_complete=workers["attribution_complete"] is True))
    else:
        result = analyze(**request)
    print(json.dumps(result))


if __name__ == "__main__":
    main()
