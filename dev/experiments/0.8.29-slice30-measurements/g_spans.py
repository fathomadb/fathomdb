"""Sample-level inclusive span trees; missing attribution is never zero work."""

from collections import defaultdict


_IDENTITY = ("engine_id", "session_id", "operation_id")


def summarize_spans(records, *, dropped_records=0, issues=(), required_phases=()):
    """Describe a batch in ns, subtracting only complete disjoint direct children.

    Inclusive error wall time is retained when observed. Errors, unavailable or
    missing observations and collection loss prevent exclusive remainders.
    Overlapping siblings remain a complete nonadditive tree, not summed shares.
    Remainders include all unallocated parent work and observation overhead;
    they are never corrected product latency or exclusive GPU service time.
    """
    records = list(records)
    problems = list(issues)
    if dropped_records:
        problems.append(f"dropped_records:{dropped_records}")
    if not records:
        problems.append("no_records")
    groups = defaultdict(list)
    indexed = {}
    children = defaultdict(list)
    spans = []
    for record in records:
        identity = tuple(record.get(field) for field in _IDENTITY)
        if any(value is None for value in identity):
            problems.append("missing_clock_identity")
        key = (*identity, record.get("span_id"))
        if key in indexed:
            problems.append("duplicate_span_identity")
        groups[identity].append(record)
        indexed[key] = record
        parent = record.get("parent_span_id")
        if parent is not None:
            children[(*identity, parent)].append(record)
        start, end = record.get("started_ns"), record.get("ended_ns")
        status = record.get("status")
        inclusive = None
        if status not in ("not_run", "unavailable"):
            if isinstance(start, int) and isinstance(end, int) and 0 <= start <= end:
                inclusive = end - start
            else:
                problems.append("missing_or_invalid_interval")
        if status not in ("ok", "not_run"):
            problems.append(f"span_status:{status}")
        spans.append({**record, "inclusive_ns": inclusive,
                      "exclusive_remainder_ns": None, "additivity": "unavailable"})
    for identity, group in groups.items():
        present = {record.get("phase") for record in group}
        for phase in required_phases:
            if phase not in present:
                problems.append(f"missing_required_phase:{phase}")
    if not records:
        problems.extend(f"missing_required_phase:{phase}" for phase in required_phases)
    for key, direct in children.items():
        parent = indexed.get(key)
        if parent is None:
            problems.append("missing_parent_span")
            continue
        parent_start, parent_end = parent.get("started_ns"), parent.get("ended_ns")
        for child in direct:
            start, end = child.get("started_ns"), child.get("ended_ns")
            if start is None or end is None:
                continue
            if parent_start is None or parent_end is None:
                problems.append("parent_interval_unavailable")
            elif start < parent_start or end > parent_end:
                problems.append("child_outside_parent")
    complete = not problems
    if complete:
        for row in spans:
            if row["status"] != "ok" or row["inclusive_ns"] is None:
                continue
            key = (*(row[field] for field in _IDENTITY), row["span_id"])
            direct = [child for child in children[key] if child["status"] != "not_run"]
            intervals = sorted((child["started_ns"], child["ended_ns"]) for child in direct)
            if any(right[0] < left[1] for left, right in zip(intervals, intervals[1:])):
                row["additivity"] = "non_additive"
                continue
            row["exclusive_remainder_ns"] = row["inclusive_ns"] - sum(
                end - start for start, end in intervals)
            row["additivity"] = "nonoverlapping_children"
    return {"complete": complete, "issues": list(dict.fromkeys(problems)), "spans": spans}
