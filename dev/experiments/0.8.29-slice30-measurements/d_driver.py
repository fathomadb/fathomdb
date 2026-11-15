"""D fixed-table driver seam; live supervision and scientific runtime remain gated."""

import json
from contextlib import ExitStack
from pathlib import Path
import time

from d_controller import run_initial_quartet, run_required
from d_attempt import run_owned_cell
from d_report import bind_execution
from d_overlap import _interval


def _json_rows(stream):
    """Decode retained frames once in native append order."""
    for line in stream:
        yield json.loads(line)


def _retained_observation(metadata, identity, streams):
    """Hydrate the capture's concrete streams without mutating its retained header."""
    observed = dict(metadata)
    issues = list(metadata["issues"]) if isinstance(metadata.get("issues"), list) else ["collection_issues_unavailable"]
    observed["issues"] = issues
    owner = {field: metadata.get(field) for field in ("engine_id", "session_id")}
    for kind, expected_count in (("runtime", "runtime_samples"), ("pending", "readiness_samples"), ("wal", "wal_samples")):
        count, pending = 0, []
        stream = streams.enter_context(Path(metadata[kind+"_path"]).open(encoding="utf-8"))
        for sample in _json_rows(stream):
            count += 1
            valid = (isinstance(sample, dict) and sample.get("cell") == identity and _interval(sample) is not None
                     and all(sample.get(field) == value for field, value in owner.items()) and not sample.get("error"))
            if kind == "runtime":
                state = sample.get("state") if isinstance(sample, dict) else None
                valid = (valid and isinstance(state, dict)
                         and all(type(state.get(field)) is int and state[field] >= 0 for field in (
                             "active_jobs", "active_scans", "queued_jobs", "in_flight_count"))
                         and all(type(state.get(field)) is bool for field in ("frozen", "pending_scan", "stopping"))
                         and state.get("scheduler_runtime_threads") == 2 and state.get("embedder_pool_size") == 5
                         and state.get("admission_capacity") == 128)
            elif kind == "pending":
                valid = (valid and type(sample.get("pending_count")) is int and sample["pending_count"] >= 0)
                pending.append(sample)
            else:
                valid = (valid and type(sample.get("db_bytes")) is int and sample["db_bytes"] > 0
                         and type(sample.get("wal_exists")) is bool
                         and type(sample.get("wal_bytes")) is int and sample["wal_bytes"] >= 0)
            if not valid:
                issues.append(kind+"_sample_unavailable")
        if not count or metadata.get(expected_count) != count:
            issues.append(kind+"_sample_count_unavailable")
        if kind == "pending":
            observed["pending"] = pending
    record_stream = streams.enter_context(Path(metadata["records_path"]).open(encoding="utf-8"))
    observed["records"] = _json_rows(record_stream)
    observed["issues"] = list(dict.fromkeys(issues))
    return observed


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
    try:
        with ExitStack() as streams:
            if cell["layer"] == "C" and isinstance(observation, dict) and "records_path" in observation:
                observation = _retained_observation(observation, identity, streams)
            report = bind_execution(identity, native, observation)
    except (OSError, ValueError, TypeError, KeyError) as error:
        report = bind_execution(identity, native, None)
        report["issues"].append("retained_stream_unavailable")
        report["stream_error"] = f"{type(error).__name__}: {error}"
    if cell["layer"] == "C" and isinstance(observation, dict) and isinstance(observation.get("issues"), list):
        report["issues"] = list(dict.fromkeys(report["issues"]+observation["issues"]))
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


def run_accounted_stage(spec, records, budget, execute, *, initial=True, clock=time.monotonic_ns, publish=None):
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
        if publish:
            publish()
        if budget.get("inflight"):
            return {"status": "not_run_ownership", "reason": "owned attempt cleanup is unresolved"}
        spent = budget.get("cell_used_ns", {}).get(cell["id"], 0)
        remaining = min(cell["cell_ns"] - spent, budget["shared_ns"] - budget["used_ns"])
        if remaining <= 0 or budget["retained_bytes"] >= budget["storage_limit"]:
            return {"status": "not_run_cap_feasibility", "reason": "remaining shared/cell allowance exhausted"}
        budget["inflight"] = {"cell": cell["id"], "started_monotonic_ns": started}
        if publish:
            publish()
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
        elif row.get("status") == "complete" and ended >= started + remaining:
            row.update(status="incomplete", deadline_exceeded=True,
                       error=row.get("error") or "whole-cell deadline reached including report qualification")
        records[cell["id"]] = row
        if publish:
            publish()
        return row

    run = run_initial_quartet if initial else run_required
    try:
        return run(spec, records, budget, prepare)
    finally:
        charge(clock())
        if publish:
            publish()


def run_supervised_stage(spec, records, budget, root, command, *, publish=None, initial=True, review=None):
    """Connect fixed-table accounting to owned children under caller-owned custody.

    The caller supplies the existing locked allocation and durable publisher;
    this callable grants no live admission or expanded profile. Every required
    row remains visible, with the initial held-quartet stop as the default.
    """
    if not initial:
        return [{"id": cell["id"], **records[cell["id"]]} if cell["id"] in records else
                {"id": cell["id"], "status": "not_run_stage_review",
                 "reason": reviewed_quartet(spec, records, budget, review)["reason"]}
                for cell in spec["cells"]]

    def execute(cell, deadline):
        return run_owned_cell(cell, root, command, deadline)

    return run_accounted_stage(spec, records, budget, execute, initial=True, publish=publish)


def reviewed_quartet(spec, records, budget, review):
    """Assess an explicit review for exactly one next required D size/mode quartet.

    Current qualified prefix identities, conservative layer bounds and shared
    headroom must remain available. Unknown cost is held; this admits no runtime
    by itself and never authorizes all remaining24 obligations automatically.
    """
    return {"status": "not_run_stage_review", "reason": "D continuation review binding is not implemented"}
