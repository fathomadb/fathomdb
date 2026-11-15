"""M client outcome populations; no exclusive timing or numerical verdict."""

from collections import Counter
import re

from metrics import aggregate, distribution


def summarize_invocations(records, wall_ns, *, stderr=(), log_level="warn"):
    """Retain every submission; logged waits describe admitted events only."""
    records = list(records)
    completed = [row for row in records if row["status"] == "completed"]
    result = aggregate(completed, wall_ns)
    success = sum(row["ok"] is True for row in completed)
    result.update(submitted=len(records), completed=len(completed), success=success,
                  requests=len(records),
                  throughput_all_per_s=len(records) * 10**9 / wall_ns if wall_ns else None,
                  unresolved=len(records) - len(completed),
                  cancellation_requested=sum(row.get("cancellation_requested", False)
                                             for row in records))
    for label, count in (("submitted", len(records)), ("completed", len(completed)),
                         ("success", success)):
        result[f"throughput_{label}_per_s"] = count * 10**9 / wall_ns if wall_ns else None
    available = log_level in ("debug", "trace")
    waits = []
    if available:
        for line in stderr:
            if "admitted after a wait" in line:
                match = re.search(r"\bwaited_ms=(\d+)\b", line)
                if match:
                    waits.append(int(match[1]) * 10**6)
    result["queue"] = {
        "availability": "observed_admitted_waits_only" if available else "unavailable",
        "events": len(waits) if available else None,
        "whole_process_submitted": len(records),
        "phase_submitted": dict(Counter(row.get("phase", "control") for row in records)),
        "conditional_wait_latency": distribution(waits) if available else None,
        "clock_resolution": "integer_milliseconds" if available else None,
    }
    return result
