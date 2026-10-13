"""Explicit dispositions for the governed cross-SDK benchmark surface."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


_ROOT = Path(__file__).resolve().parents[1]
_PARITY = _ROOT / "src/conformance/governed-operation-parity.json"

_MEASURE = {
    "engine.search_text_only",
    "read.get",
    "graph.neighbors",
}
_SETUP = {
    "engine.open",
    "admin.configure",
    "engine.write",
    "engine.actuate",
    "engine.register_source_dependency",
    "engine.transition",
    "engine.purge",
    "engine.erase_source",
    "engine.freeze_read_context",
    "engine.close",
    "engine.configure_projections",
    "engine.ingest_with_extractor",
    "engine.consolidate_with_provider",
}
_OBSERVE = {
    "engine.dependencies_for_source",
    "engine.dependency_for_derived",
    "engine.read_dependency_closure",
    "engine.trace_dependency",
    "read.collection",
    "read.mutations",
    "read.list",
    "read.crossed_boundary_since",
    "read.projections",
    "read.projection_status",
    "read.embedding_readiness",
    "read.projection_generation_status",
    "read.mutation_projection_status",
    "read.canonical_page",
    "read.operational_state",
    "read.operational_state_page",
    "read.get_many",
}
_EXCLUDED = {
    "engine.search": "configured hybrid fixture and model oracle pending",
    "engine.search_frozen": "frozen-context fixture pending",
    "engine.search_expand_frozen": "frozen graph-evidence fixture pending",
    "engine.search_projected_text": "named projection fixture pending",
    "engine.search_with_evidence": "evidence oracle pending",
    "engine.resolve_evidence": "evidence resolution fixture pending",
    "engine.resolve_graph_evidence": "graph evidence resolution fixture pending",
    "graph.expand": "bounded expansion fixture pending",
    "graph.search_expand": "search-expansion fixture pending",
    "rerank": "pinned reranker artifact and oracle pending",
    "engine.embed": "pinned embedder artifact and oracle pending",
}

_EXTRAS: dict[str, tuple[str, str, str, str]] = {
    "engine.drain": ("drain", "drain", "drain", "setup"),
    "engine.open_report": ("open_report", "openReport", "open_report", "observe"),
    "engine.config": ("config", "config", "config", "observe"),
    "engine.counters": ("counters", "counters", "counters", "observe"),
    "engine.dense_disabled": (
        "dense_disabled",
        "denseDisabled",
        "dense_disabled",
        "observe",
    ),
    "engine.dense_disabled_reason": (
        "dense_disabled_reason",
        "denseDisabledReason",
        "dense_disabled_reason",
        "observe",
    ),
    "engine.vector_equivalence_refusal_count": (
        "vector_equivalence_refusal_count",
        "vectorEquivalenceRefusalCount",
        "vector_equivalence_refusal_count",
        "observe",
    ),
    "engine.enable_telemetry": (
        "enable_telemetry",
        "enableTelemetry",
        "enable_telemetry",
        "setup",
    ),
    "engine.last_telemetry_query_id": (
        "last_telemetry_query_id",
        "lastTelemetryQueryId",
        "last_telemetry_query_id",
        "observe",
    ),
    "engine.record_feedback": (
        "record_feedback",
        "recordFeedback",
        "record_feedback",
        "setup",
    ),
    "engine.set_profiling": ("set_profiling", "setProfiling", "set_profiling", "setup"),
    "engine.set_slow_threshold_ms": (
        "set_slow_threshold_ms",
        "setSlowThresholdMs",
        "set_slow_threshold_ms",
        "setup",
    ),
    "embed_batch_cls": (
        "embed_batch_cls",
        "embedBatchCls",
        "embed_batch_cls",
        "excluded_with_reason",
    ),
}


def load_registry(parity_path: str | Path = _PARITY) -> dict[str, dict[str, Any]]:
    """Check every governed ID against its explicit benchmark disposition."""
    parity = json.loads(Path(parity_path).read_text(encoding="utf-8"))
    if parity.get("schema_version") != "fathomdb.governed-operation-parity/v1":
        raise ValueError("SDK parity schema mismatch")
    groups = [_MEASURE, _SETUP, _OBSERVE, set(_EXCLUDED)]
    if sum(map(len, groups)) != len(set().union(*groups)):
        raise ValueError("SDK registry has duplicate dispositions")
    declared = set().union(*groups)
    operations = {
        item["id"]: item for item in parity["operations"] if item["state"] == "live"
    }
    if set(operations) != declared:
        raise ValueError("SDK registry drift against live governed operations")
    registry: dict[str, dict[str, Any]] = {}
    for operation_id, item in operations.items():
        disposition = (
            "measure"
            if operation_id in _MEASURE
            else "setup"
            if operation_id in _SETUP
            else "observe"
            if operation_id in _OBSERVE
            else "excluded_with_reason"
        )
        registry[operation_id] = {
            "semantic_version": 1,
            "disposition": disposition,
            "bindings": {sdk: item[sdk] for sdk in ("python", "typescript", "rust")},
            "reason": _EXCLUDED.get(operation_id),
            "input_schema": "sdk.driver.v1" if disposition == "measure" else None,
            "timing_boundary": "sdk_call_to_materialized_result"
            if disposition == "measure"
            else None,
            "normalizer": "ordered_ids" if disposition == "measure" else None,
            "readiness": "fresh_database_drained" if disposition == "measure" else None,
        }
    for operation_id, (python, typescript, rust, disposition) in _EXTRAS.items():
        registry[operation_id] = {
            "semantic_version": 1,
            "disposition": disposition,
            "bindings": {
                "python": {"locator": "public", "spelling": python},
                "typescript": {"locator": "public", "spelling": typescript},
                "rust": {"locator": "public", "spelling": rust},
            },
            "reason": "pinned model artifact pending"
            if operation_id == "embed_batch_cls"
            else None,
            "input_schema": None,
            "timing_boundary": None,
            "normalizer": None,
            "readiness": None,
        }
    return registry
