"""WP-40D (Gate1-D10 / R32-08) — Python PerHitExplain.vector_cosine parity & validation."""

from __future__ import annotations

import math
import os
from pathlib import Path
from types import SimpleNamespace

import pytest

from fathomdb import (
    Engine,
    EvidenceSearchRequestV1,
    PerHitExplain,
    ProjectionSpec,
    ReadContextV1,
    ReadView,
    SearchFilter,
)
from fathomdb.engine import _map_per_hit_explain


def _fake_native_per_hit(**overrides: object) -> SimpleNamespace:
    base: dict[str, object] = {
        "id": 1,
        "arm": "vector",
        "vector_rank": 0,
        "text_rank": None,
        "graph_rank": None,
        "fused_score": 0.016,
        "ce_score": None,
        "blended": 0.016,
        "importance": None,
        "confidence": None,
        "graph_seed_source": None,
        "graph_hop_depth": None,
        "vector_cosine": 0.875,
    }
    base.update(overrides)
    return SimpleNamespace(**base)


@pytest.mark.parametrize("raw,expected", [
    (None, None),
    (0.0, 0.0),
    (1.0, 1.0),
    (-1.0, -1.0),
    (0.42, 0.42),
    (0, 0.0),
    (1, 1.0),
    (-1, -1.0),
])
def test_map_per_hit_explain_accepts_valid_vector_cosine(
    raw: float | int | None,
    expected: float | None,
) -> None:
    mapped = _map_per_hit_explain(_fake_native_per_hit(vector_cosine=raw))
    assert isinstance(mapped, PerHitExplain)
    if expected is None:
        assert mapped.vector_cosine is None
    else:
        assert mapped.vector_cosine == expected
        assert isinstance(mapped.vector_cosine, float)


def test_map_per_hit_explain_absent_vector_cosine_defaults_none() -> None:
    native = _fake_native_per_hit()
    delattr(native, "vector_cosine")
    mapped = _map_per_hit_explain(native)
    assert mapped.vector_cosine is None


@pytest.mark.parametrize(
    "bad_value",
    [
        True,
        False,
        float("nan"),
        float("inf"),
        float("-inf"),
        1.5,
        -1.5,
        1.00001,
        -1.00001,
        "0.5",
    ],
)
def test_map_per_hit_explain_rejects_invalid_vector_cosine(bad_value: object) -> None:
    native = _fake_native_per_hit(vector_cosine=bad_value)
    with pytest.raises(ValueError, match="/vectorCosine"):
        _map_per_hit_explain(native)


def test_per_hit_explain_dataclass_has_vector_cosine_default_none() -> None:
    assert "vector_cosine" in PerHitExplain.__dataclass_fields__
    hit = PerHitExplain(
        id=1,
        arm="text",
        vector_rank=None,
        text_rank=0,
        graph_rank=None,
        fused_score=0.5,
        ce_score=None,
        blended=0.5,
    )
    assert hit.vector_cosine is None


def test_text_only_search_and_evidence_expose_none_vector_cosine(tmp_path: Path) -> None:
    db_path = tmp_path / "wp40d-text-only.fdb"
    engine = Engine.open(str(db_path), use_default_embedder=False)
    try:
        engine.write([
            {
                "kind": "doc",
                "body": "quasar nebula pulsar text record",
                "source_id": "src:py-text",
                "logical_id": "py-text-1",
                "provenance": {
                    "schema_version": 1,
                    "role": "canonical",
                    "artifact_revision_id": "rev-py-text-1",
                    "source_version_id": "ver-py-text-1",
                },
            }
        ])
        res = engine.search("quasar nebula", explain=True)
        assert res.explanation is not None
        assert len(res.explanation.per_hit) == 1
        entry = res.explanation.per_hit[0]
        assert entry.vector_rank is None
        assert entry.vector_cosine is None

        frozen = engine.freeze_read_context(
            ReadContextV1(view=ReadView(), eligibility=SearchFilter())
        )
        ev = engine.search_with_evidence(
            EvidenceSearchRequestV1(
                query="quasar nebula",
                context=frozen,
                include_explanation=True,
            )
        )
        assert ev.search_result.explanation is not None
        assert len(ev.search_result.explanation.per_hit) == 1
        assert ev.search_result.explanation.per_hit[0].vector_rank is None
        assert ev.search_result.explanation.per_hit[0].vector_cosine is None
    finally:
        engine.close()


@pytest.mark.skipif(
    bool(os.environ.get("FATHOMDB_SKIP_NETWORK_TESTS")),
    reason="default embedder weight loading skipped under FATHOMDB_SKIP_NETWORK_TESTS",
)
def test_vector_scored_search_and_evidence_expose_finite_vector_cosine(
    tmp_path: Path,
) -> None:
    db_path = tmp_path / "wp40d-vector.fdb"
    engine = Engine.open(str(db_path), use_default_embedder=True)
    try:
        engine.configure_projections(
            [
                ProjectionSpec(
                    name="memory",
                    roles=frozenset({"searchable"}),
                    vector=True,
                )
            ],
            [],
        )
        engine.write([
            {
                "kind": "doc",
                "body": "distributed database transaction isolation",
                "source_id": "src:py-vec",
                "logical_id": "py-vec-1",
                "provenance": {
                    "schema_version": 1,
                    "role": "canonical",
                    "artifact_revision_id": "rev-py-vec-1",
                    "source_version_id": "ver-py-vec-1",
                },
            }
        ])
        engine.drain(timeout_s=30)

        res = engine.search("distributed database transaction", explain=True)
        assert res.explanation is not None
        assert len(res.explanation.per_hit) >= 1
        for p in res.explanation.per_hit:
            if p.vector_rank is not None:
                assert p.vector_cosine is not None
                assert math.isfinite(p.vector_cosine)
                assert -1.0 <= p.vector_cosine <= 1.0
            else:
                assert p.vector_cosine is None

        frozen = engine.freeze_read_context(
            ReadContextV1(view=ReadView(), eligibility=SearchFilter())
        )
        ev = engine.search_with_evidence(
            EvidenceSearchRequestV1(
                query="distributed database transaction",
                context=frozen,
                include_explanation=True,
            )
        )
        assert ev.search_result.explanation is not None
        assert any(
            p.vector_rank is not None
            and p.vector_cosine is not None
            and -1.0 <= p.vector_cosine <= 1.0
            for p in ev.search_result.explanation.per_hit
        )
    finally:
        engine.close()
