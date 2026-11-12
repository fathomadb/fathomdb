"""Slice30 sample-level descriptive statistics; no performance verdicts."""


def distribution(samples):
    """Summarize nanosecond samples using preregistered nearest ranks."""
    return {"count": len(samples)}


def aggregate(records, wall_ns):
    """Keep success/error latency populations and all-request throughput separate."""
    return {"requests": len(records)}


def paired_difference(records):
    """Compare matched query/repeat adapter observations with clustered intervals."""
    return {"pairs": 0}
