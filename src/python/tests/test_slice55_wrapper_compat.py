"""Slice 55 RED construction-compatibility and native-presence contracts."""

from types import SimpleNamespace

from fathomdb.engine import _map_per_hit_explain
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
