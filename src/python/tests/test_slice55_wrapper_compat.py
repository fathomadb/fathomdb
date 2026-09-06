"""Slice 55 RED construction-compatibility and native-presence contracts."""

import copy
from collections.abc import Callable
from types import SimpleNamespace

import pytest

from fathomdb.engine import _map_native_search_result, _map_per_hit_explain
from fathomdb.types import Explanation, PerHitExplain, QueryTrace


def _trace() -> QueryTrace:
    return QueryTrace(
        query_chars=1,
        k=1,
        rerank_depth=0,
        pool_n=0,
        alpha=0.3,
        use_graph_arm=False,
        recency=False,
        embedder_id="",
        ce_active=False,
        vector_hits=0,
        text_hits=1,
        graph_hits=0,
        dropped_edge_hits=0,
    )


def test_slice55_old_python_construction() -> None:
    hit = PerHitExplain(1, "text", None, 0, None, 1.0, None, 1.0)
    explanation = Explanation(_trace(), [hit])
    assert explanation.correlation_id == ""
    assert hit.structural is None


def test_slice55_absent_native_fields_use_legacy_defaults() -> None:
    native = SimpleNamespace(
        id=1,
        arm="text",
        vector_rank=None,
        text_rank=0,
        graph_rank=None,
        fused_score=1.0,
        ce_score=None,
        blended=1.0,
        importance=None,
        confidence=None,
    )
    assert _map_per_hit_explain(native).structural is None


def test_slice55_direct_candidate_native_presence_fixture() -> None:
    native = SimpleNamespace(
        id=1,
        arm="text",
        vector_rank=None,
        text_rank=0,
        graph_rank=None,
        fused_score=1.0,
        ce_score=None,
        blended=1.0,
        importance=None,
        confidence=None,
        structural=SimpleNamespace(
            schema_version=1,
            inclusion_state="included",
            projection_origin="synchronous_body_fts",
            dependency_state="not_applicable",
            lifecycle_state="node_active",
            degradation_codes=[],
        ),
    )
    mapped = _map_per_hit_explain(native)
    assert mapped.structural is not None
    assert mapped.structural.schema_version == 1


@pytest.mark.parametrize(
    ("structural", "path"),
    [
        (
            SimpleNamespace(
                schema_version=2,
                inclusion_state="included",
                projection_origin="synchronous_body_fts",
                dependency_state="not_applicable",
                lifecycle_state="node_active",
                degradation_codes=[],
            ),
            "/structural/schemaVersion",
        ),
        (
            SimpleNamespace(
                schema_version=1,
                inclusion_state="included",
                projection_origin="synchronous_body_fts",
                dependency_state="not_applicable",
                lifecycle_state="node_active",
                degradation_codes=["projection_blocked"],
            ),
            "/structural/inclusionState",
        ),
        (
            SimpleNamespace(
                schema_version=1,
                inclusion_state="degraded",
                projection_origin="synchronous_body_fts",
                dependency_state="not_applicable",
                lifecycle_state="node_active",
                degradation_codes=["projection_blocked", "projection_blocked"],
            ),
            "/structural/degradationCodes/1",
        ),
    ],
)
def test_slice55_python_recursively_validates_structural_explanation(
    structural: SimpleNamespace, path: str
) -> None:
    native = SimpleNamespace(
        id=1,
        arm="text",
        vector_rank=None,
        text_rank=0,
        graph_rank=None,
        fused_score=1.0,
        ce_score=None,
        blended=1.0,
        importance=None,
        confidence=None,
        structural=structural,
    )
    with pytest.raises(ValueError, match=f"invalid explanation response at {path}"):
        _map_per_hit_explain(native)


def _candidate_native_search_result() -> SimpleNamespace:
    structural = SimpleNamespace(
        schema_version=1,
        inclusion_state="included",
        projection_origin="synchronous_body_fts",
        dependency_state="not_applicable",
        lifecycle_state="node_active",
        degradation_codes=[],
    )
    per_hit = SimpleNamespace(
        id=7,
        arm="text",
        vector_rank=None,
        text_rank=0,
        graph_rank=None,
        fused_score=0.5,
        ce_score=None,
        blended=1.0,
        importance=None,
        confidence=None,
        structural=structural,
    )
    trace = SimpleNamespace(
        query_chars=6,
        k=1,
        rerank_depth=0,
        pool_n=0,
        alpha=0.3,
        use_graph_arm=False,
        recency=False,
        embedder_id="",
        ce_active=False,
        vector_hits=0,
        text_hits=1,
        graph_hits=0,
        dropped_edge_hits=0,
    )
    hit = SimpleNamespace(
        id=SimpleNamespace(space="logical", value="hit-1"),
        kind="note",
        body="needle",
        score=1.0,
        branch="text",
        source_id="source-1",
        ce_score=None,
    )
    return SimpleNamespace(
        projection_cursor=1,
        soft_fallback=None,
        results=[hit],
        explanation=SimpleNamespace(
            trace=trace,
            per_hit=[per_hit],
            correlation_id="x0123456789abcdef0123456789abcdef-0",
        ),
    )


def _duplicate_candidate_id(value: SimpleNamespace) -> None:
    value.results.append(copy.deepcopy(value.results[0]))
    value.results[1].id.value = "hit-2"
    value.explanation.per_hit.append(copy.deepcopy(value.explanation.per_hit[0]))


@pytest.mark.parametrize(
    ("mutate", "path"),
    [
        (lambda value: setattr(value.explanation.trace, "query_chars", True), "/trace/queryChars"),
        (lambda value: setattr(value.explanation.trace, "k", 0), "/trace/k"),
        (lambda value: setattr(value.explanation.trace, "alpha", float("nan")), "/trace/alpha"),
        (lambda value: setattr(value.explanation, "correlation_id", ""), "/correlationId"),
        (lambda value: delattr(value.explanation.per_hit[0], "structural"), "/perHit/0/structural"),
        (lambda value: value.explanation.per_hit.clear(), "/perHit"),
        (lambda value: setattr(value.explanation.per_hit[0], "arm", "vector"), "/perHit/0/arm"),
        (lambda value: setattr(value.explanation.per_hit[0], "blended", 0.5), "/perHit/0/blended"),
        (lambda value: setattr(value.results[0], "branch", "future_arm"), "/results/0/branch"),
        (_duplicate_candidate_id, "/perHit/1/id"),
    ],
)
def test_slice55_candidate_native_explanation_is_strict_and_coherent(
    mutate: Callable[[SimpleNamespace], None], path: str
) -> None:
    native = _candidate_native_search_result()
    mutate(native)
    with pytest.raises(ValueError, match=f"invalid explanation response at {path}"):
        _map_native_search_result(native)
