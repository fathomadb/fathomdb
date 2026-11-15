"""Bind retained D observations to one execution; native capture remains separate."""

from bisect import bisect_right
from collections import defaultdict

from d_overlap import _interval, classify_overlap
from g_spans import summarize_spans


_UPPER = 1 << 63
_COMMIT = ("projection_commit_gate_wait", "projection_commit_begin", "projection_commit_prepare",
           "projection_commit_publication", "projection_commit_sqlite", "projection_commit_completion")
_FACTS = ("projection_dequeue", "projection_commit_outcome")
_ZERO = ("input_failure", "input_deferred", "generation_skips", "member_skips", "terminal_skips",
         "staged_failure", "committed_failure")


def _cursors(record):
    values = record.get("facts", {}).get("cursors")
    if (not isinstance(values, list) or any(type(value) is not int or value < 1 for value in values)
            or len(set(values)) != len(values)):
        return None
    return set(values)


def _ordinary(cell, actual):
    issues, cursors, operations = [], set(), {}
    n = cell.get("n")
    if (actual.get("cell") != cell or actual.get("complete") is not True
            or actual.get("close_succeeded") is not True or type(n) is not int or not 0 < n <= 300000):
        issues.append("execution_or_close_unavailable")
    for batch in actual.get("batches") or []:
        count, cursor = batch.get("rows"), batch.get("cursor")
        if type(count) is not int or not 0 < count <= 1000 or type(cursor) is not int or cursor < count:
            issues.append("commit_receipt_unavailable")
            continue
        # WriteReceipt's documented batch cursor range is one coordinate per input row.
        rows = set(range(cursor-count+1, cursor+1))
        if rows & cursors:
            issues.append("duplicate_governed_cursor")
        cursors.update(rows)
    if len(cursors) != n:
        issues.append("governed_cursor_coverage")
    terminal = actual.get("final") or {}
    if (not terminal.get("generation_id") or any(terminal.get(field) != n for field in (
            "natural_rows", "physical_rows", "vector_rows", "span_rows", "terminal_rows"))
            or any(terminal.get(field) != 0 for field in (
                "pending_count", "generation_pending_count", "generation_failed_count", "failed_rows"))):
        issues.append("physical_or_generation_closure_unavailable")
    runtime = actual.get("runtime") or {}
    if any(runtime.get(field) != count for field, count in (
            ("scheduler_workers", 2), ("embed_workers", 5), ("admission_capacity", 128))):
        issues.append("runtime_defaults_unavailable")
    calls = actual.get("calls") or []
    if len(calls) != 1000:
        issues.append("natural_call_count")
    previous = 0
    for index, call in enumerate(calls):
        bounds = _interval(call)
        if (call.get("sample_index") != index or call.get("query_index") != index % 500
                or call.get("route") != "engine_hybrid" or call.get("limit") != 10
                or call.get("ok") is not True or bounds is None or bounds[0] < previous
                or bounds[0] == bounds[1]):
            issues.append("natural_call_unavailable")
        if bounds:
            previous = bounds[1]
        if cell.get("layer") == "C":
            operation = call.get("operation_id")
            if type(operation) is not int or not 0 < operation < _UPPER or operation in operations:
                issues.append("natural_operation_unavailable")
            else:
                operations[operation] = bounds
    return issues, cursors, operations


def _add_phases(phases, records, issues, dropped):
    tree = summarize_spans(records, dropped_records=dropped)
    issues.extend(tree["issues"])
    for row in tree["spans"]:
        entry = phases[row["phase"]]
        entry["count"] += 1
        for field in ("inclusive_ns", "exclusive_remainder_ns"):
            if row[field] is not None:
                entry[field+"_sum"] += row[field]
                entry[field+"_count"] += 1
        entry["non_additive_count"] += row["additivity"] == "non_additive"


def _group(records, scans, generation, phases, issues, dropped):
    by_phase = defaultdict(list)
    if any(record.get("status") != "ok" for record in records):
        issues.append("required_group_phase_unobserved")
    for record in records:
        by_phase[record.get("phase")].append(record)
    roots = by_phase["projection_processing"]
    root = roots[0] if len(roots) == 1 else {}
    cursors = _cursors(root)
    if not cursors or len(cursors) > 64:
        issues.append("processing_cursors_unavailable")
    for phase in (*_COMMIT, "projection_dequeue", "projection_commit_outcome"):
        if len(by_phase[phase]) != 1 or _cursors(by_phase[phase][0]) != cursors:
            issues.append("group_phase_coverage:"+phase)
    dequeues = by_phase["projection_dequeue"]
    if len(dequeues) == 1:
        source = scans.get(dequeues[0].get("facts", {}).get("scan_operation_id"))
        if source is None or cursors is None or not cursors <= source:
            issues.append("group_scan_coverage")
    embedded = set()
    for record in by_phase["projection_embed_dispatch_wall"]:
        selected = _cursors(record)
        if selected is None or cursors is None or not selected <= cursors:
            issues.append("group_embed_coverage")
        else:
            embedded.update(selected)
    if not embedded or embedded != cursors:
        issues.append("group_embed_coverage")
    timed = []
    for record in records:
        if record.get("phase") in _FACTS:
            if record.get("started_ns") is not None or record.get("ended_ns") is not None:
                issues.append("untimed_fact_has_interval")
        else:
            timed.append(record)
    _add_phases(phases, timed, issues, dropped)
    outcomes = by_phase["projection_commit_outcome"]
    facts = outcomes[0].get("facts", {}) if len(outcomes) == 1 else {}
    committed = (cursors is not None and facts.get("generation_id") == generation
                 and facts.get("commit_attempted") is True and facts.get("publication_state") == "committed"
                 and all(type(facts.get(field)) is int and facts[field] == len(cursors)
                         for field in ("input_success", "staged_success", "committed_success"))
                 and all(type(facts.get(field)) is int and facts[field] == 0 for field in _ZERO))
    if not committed:
        issues.append("durable_group_outcome_unavailable")
    return root, cursors if committed else None


def _overlap(cell, owner, queries, workers, pending):
    if not queries or not workers or not pending:
        return {"verdict": "INCOMPLETE", "reason": "required exact-execution intervals/pending are unavailable"}
    ordered_queries = sorted(queries, key=lambda record: record["started_ns"])
    starts = [record["started_ns"] for record in ordered_queries]
    ordered_workers = sorted(workers, key=lambda record: record["started_ns"])
    best, index = None, 0
    for sample in sorted(pending, key=lambda row: row["started_ns"]):
        if sample["pending_count"] <= 0:
            continue
        while index < len(ordered_workers) and ordered_workers[index]["started_ns"] <= sample["started_ns"]:
            worker = ordered_workers[index]
            if best is None or worker["ended_ns"] > best["ended_ns"]:
                best = worker
            index += 1
        query_index = bisect_right(starts, sample["started_ns"])-1
        if (query_index < 0 or best is None or best["ended_ns"] < sample["ended_ns"]
                or ordered_queries[query_index]["ended_ns"] < sample["ended_ns"]):
            continue
        query = ordered_queries[query_index]
        witness = classify_overlap({"cell": cell, **owner, "operation_id": query["operation_id"],
            "records": [{"cell": cell, "record": record} for record in (query, best)],
            "pending": [sample], "dropped_records": 0, "issues": []})
        if witness["verdict"] == "OBSERVED_OVERLAP":
            return witness
    return {"verdict": "NO_OBSERVED_OVERLAP", "reason": "no positive pending bracket fits an actual call/worker intersection"}


def bind_execution(cell, actual, observation=None):
    """Bind current receipts, physical closure, natural calls and worker facts.

    Input records follow retained take/append order. Worker groups are reduced
    when their processing root closes; untimed facts are never duration spans.
    Phase means describe inclusive host time and demonstrated tree remainders.
    T ignores C records and remains without worker/overlap attribution. This pure
    qualification helper neither runs a cell nor grants a campaign/profile GO.
    """
    base, expected, operations = _ordinary(cell, actual)
    result = {"cell": cell, "complete": False, "observations_complete": None,
              "committed_success_rows": None, "committed_cursors": None, "worker_groups": None,
              "actual_overlap": None, "overlap": None, "phases": {}, "issues": base}
    if cell.get("layer") == "T":
        if (actual.get("collection_enabled") is not False or actual.get("periodic_numeric_polls") != 0
                or actual.get("periodic_readiness_polls") != 0):
            base.append("ordinary_collection_or_polling_enabled")
        result.update(complete=not base, overlap={"verdict": "ACTUAL_OVERLAP_UNOBSERVED"})
        return result
    observation = observation or {}
    issues, phases, scans, groups = [], defaultdict(lambda: defaultdict(int)), {}, defaultdict(list)
    owner = {field: observation.get(field) for field in ("engine_id", "session_id")}
    dropped = observation.get("dropped_records")
    if (cell.get("layer") != "C" or actual.get("collection_enabled") is not True
            or any(type(value) is not int or value <= 0 for value in owner.values())
            or dropped != 0 or observation.get("issues") != []
            or not actual.get("periodic_numeric_polls") or not actual.get("periodic_readiness_polls")):
        issues.append("required_collection_unavailable")
    committed, queries, workers, seen_queries = set(), [], [], set()
    durable_known, worker_count = True, 0
    for frame in observation.get("records") or []:
        record = frame.get("record", {})
        if frame.get("cell") != cell or any(record.get(field) != value for field, value in owner.items()):
            issues.append("foreign_execution_record")
            continue
        phase, operation = record.get("phase"), record.get("operation_id")
        if record.get("status") not in ("ok", "not_run"):
            issues.append("record_status_unavailable")
        if phase == "api_hybrid":
            bounds, outer = _interval(record), operations.get(operation)
            if (operation in seen_queries or bounds is None or outer is None
                    or not outer[0] <= bounds[0] < bounds[1] <= outer[1] or record.get("status") != "ok"):
                issues.append("natural_api_interval_unavailable")
            else:
                queries.append(record)
                seen_queries.add(operation)
        if not isinstance(phase, str) or not phase.startswith("projection_"):
            continue
        if type(operation) is not int or operation < _UPPER:
            issues.append("projection_operation_unavailable")
        if phase == "projection_scan":
            if record.get("status") != "ok":
                issues.append("required_scan_unobserved")
            selected = _cursors(record)
            if selected is None or len(selected) > 128 or operation in scans:
                issues.append("scan_cursor_coverage_unavailable")
            else:
                scans[operation] = selected
            _add_phases(phases, [record], issues, dropped)
            continue
        groups[operation].append(record)
        if phase == "projection_processing":
            worker_count += 1
            root, cursors = _group(groups.pop(operation), scans, actual["final"].get("generation_id"),
                                   phases, issues, dropped)
            if _interval(root) is not None and root.get("status") == "ok":
                workers.append(root)
            if cursors is None or committed & cursors:
                durable_known = False
                issues.append("committed_cursor_coverage_unavailable")
            else:
                committed.update(cursors)
    if groups or seen_queries != set(operations) or not worker_count or committed != expected:
        issues.append("required_current_group_or_call_coverage")
    if not worker_count or groups:
        durable_known = False
    pending = []
    for sample in observation.get("pending") or []:
        if (sample.get("cell") != cell or any(sample.get(field) != value for field, value in owner.items())
                or _interval(sample) is None or type(sample.get("pending_count")) is not int
                or sample["pending_count"] < 0):
            issues.append("pending_observation_unavailable")
        else:
            pending.append(sample)
    if not pending:
        issues.append("durable_pending_observations_unavailable")
    witness = _overlap(cell, owner, queries, workers, pending) if cell.get("mode") == "overlap" else None
    observed = witness is not None and witness["verdict"] == "OBSERVED_OVERLAP"
    result.update(observations_complete=not issues, committed_success_rows=len(committed) if durable_known else None,
                  committed_cursors=sorted(committed) if durable_known else None, worker_groups=worker_count,
                  actual_overlap=observed if cell.get("mode") == "overlap" else None, overlap=witness)
    result["phases"] = {phase: {"count": entry["count"], "timed_count": entry["inclusive_ns_count"],
        "inclusive_mean_ns": entry["inclusive_ns_sum"]/entry["inclusive_ns_count"] if entry["inclusive_ns_count"] else None,
        "exclusive_remainder_mean_ns": entry["exclusive_remainder_ns_sum"]/entry["exclusive_remainder_ns_count"] if entry["exclusive_remainder_ns_count"] else None,
        "non_additive_count": entry["non_additive_count"]} for phase, entry in phases.items()}
    if witness is not None and not observed:
        issues.append("required_actual_overlap_unqualified")
    result["issues"] = list(dict.fromkeys(base+issues))
    result["complete"] = not result["issues"]
    return result
