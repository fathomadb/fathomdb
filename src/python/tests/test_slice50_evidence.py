"""Slice 50 source-complete evidence through the installed Python surface."""

from __future__ import annotations

import hashlib
from collections.abc import Callable
from types import SimpleNamespace

import fathomdb
import pytest  # pyright: ignore[reportMissingImports]
from fathomdb.engine import _map_native_evidence_search, _map_native_resolved_evidence
from fathomdb.errors import InvalidArgumentError


def _native_resolved_fixture() -> SimpleNamespace:
    return SimpleNamespace(
        schema_version=1,
        logical_id="claim",
        artifact_revision_id="claim-r1",
        source_id="source",
        source_version_id="source-v1",
        source_revision_id="source-r1",
        locator_kind="whole_body",
        locator_start_inclusive=None,
        locator_end_exclusive=None,
        canonical_source_body="source bytes",
        evidence_text="source bytes",
        canonical_source_hash="00",
        effective_valid_at=1,
        artifact_lifecycle_kind="node",
        artifact_lifecycle_state="active",
        artifact_superseded=False,
        artifact_valid_at_effective=None,
        source_lifecycle_state="active",
        projection_origin=SimpleNamespace(
            schema_version=1,
            artifact_class="node",
            representative_arm="text",
            projection_generation_id="generation",
            graph_origin_kind=None,
            graph_edge_artifact_revision_id=None,
            graph_hop_count=None,
        ),
        retrieval_contribution=SimpleNamespace(
            schema_version=1,
            vector_rank=None,
            text_rank=0,
            graph_rank=None,
            fused_score=1.0,
            ce_score=None,
            blended_score=1.0,
            importance=None,
            confidence=None,
        ),
        dependency=SimpleNamespace(
            schema_version=1,
            dependency_id="dependency",
            source_revision_id="source-r1",
            derived_revision_id="claim-r1",
            registered_dependency_generation="1",
        ),
    )


def test_search_and_resolve_exact_source_evidence(db_path: str) -> None:
    source_body = "canonical evidence bytes"
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write(
        [
            {
                "kind": "document",
                "body": source_body,
                "source_id": "python-evidence-source",
                "logical_id": "python-source",
                "provenance": {
                    "schema_version": 1,
                    "role": "canonical",
                    "artifact_revision_id": "python-source-r1",
                    "source_version_id": "python-version-r1",
                },
            },
            {
                "kind": "fact",
                "body": "pythonevidenceneedle",
                "source_id": "python-evidence-source",
                "logical_id": "python-claim",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "python-claim-r1",
                    "source_version_id": "python-version-r1",
                    "source_revision_id": "python-source-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": hashlib.sha256(source_body.encode()).hexdigest(),
                    },
                },
            },
        ]
    )
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    result = engine.search_with_evidence(
        fathomdb.EvidenceSearchRequestV1(
            query="pythonevidenceneedle",
            context=frozen,
            include_explanation=False,
        )
    )
    assert result.search_result.results[0].body == "pythonevidenceneedle"
    assert result.search_result.explanation is None
    assert result.evidence[0].result_index == 0
    assert result.evidence[0].artifact_revision_id == "python-claim-r1"

    resolved = engine.resolve_evidence(
        fathomdb.EvidenceResolveRequestV1(
            evidence_ref=result.evidence[0].evidence_ref,
            context=frozen,
        )
    )
    assert resolved.canonical_source_body == source_body
    assert resolved.evidence_text == source_body
    assert resolved.source_revision_id == "python-source-r1"
    assert resolved.projection_origin.representative_arm == "text"
    engine.close()


def test_unsupported_evidence_schema_is_typed(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())

    with pytest.raises(fathomdb.EvidenceError) as raised:
        engine.search_with_evidence(
            fathomdb.EvidenceSearchRequestV1(
                query="needle",
                context=frozen,
                schema_version=2,
            )
        )

    assert raised.value.reason == "unsupported_schema_version"
    assert raised.value.field_path == "/schemaVersion"

    tampered = fathomdb.FrozenReadContextV1(
        effective_valid_at=frozen.effective_valid_at,
        context=frozen.context,
        token=f"{frozen.token}0",
    )
    with pytest.raises(fathomdb.EvidenceError) as unavailable:
        engine.search_with_evidence(
            fathomdb.EvidenceSearchRequestV1(query="needle", context=tampered)
        )
    assert unavailable.value.reason == "evidence_unavailable"
    assert unavailable.value.field_path == "/evidenceRef"

    with pytest.raises(InvalidArgumentError, match="rerank_depth must be >= 0"):
        engine.search_with_evidence(
            fathomdb.EvidenceSearchRequestV1(
                query="needle",
                context=frozen,
                rerank_depth=-1,
            )
        )
    with pytest.raises(
        InvalidArgumentError, match="rerank_depth must be <= 4294967295"
    ):
        engine.search_with_evidence(
            fathomdb.EvidenceSearchRequestV1(
                query="needle",
                context=frozen,
                rerank_depth=2**32,
            )
        )
    engine.close()


def test_unknown_native_evidence_versions_fail_closed() -> None:
    for mapper, native in [
        (_map_native_evidence_search, SimpleNamespace(schema_version=2)),
        (_map_native_resolved_evidence, SimpleNamespace(schema_version=2)),
    ]:
        with pytest.raises(fathomdb.EvidenceError) as raised:
            mapper(native)
        assert raised.value.reason == "unsupported_schema_version"
        assert raised.value.field_path == "/schemaVersion"


@pytest.mark.parametrize(
    ("mutate", "field_path"),
    [
        (lambda value: setattr(value, "locator_kind", "words"), "/locator/kind"),
        (
            lambda value: setattr(value.retrieval_contribution, "fused_score", float("nan")),
            "/retrievalContribution/fusedScore",
        ),
        (
            lambda value: setattr(value.retrieval_contribution, "fused_score", None),
            "/retrievalContribution/fusedScore",
        ),
        (
            lambda value: setattr(value.retrieval_contribution, "blended_score", None),
            "/retrievalContribution/blendedScore",
        ),
        (
            lambda value: setattr(value.projection_origin, "artifact_class", "edge"),
            "/projectionOrigin/artifactClass",
        ),
        (
            lambda value: (
                setattr(value.projection_origin, "representative_arm", "graph_arm"),
                setattr(value.projection_origin, "graph_origin_kind", "entity_seed"),
            ),
            "/projectionOrigin/graphOrigin/kind",
        ),
        (
            lambda value: setattr(value.dependency, "registered_dependency_generation", "01"),
            "/dependency/registeredDependencyGeneration",
        ),
        (
            lambda value: (
                setattr(value, "locator_kind", "utf8_bytes"),
                setattr(value, "locator_start_inclusive", 2**64),
                setattr(value, "locator_end_exclusive", 2**64),
            ),
            "/locator/startInclusive",
        ),
        (
            lambda value: (
                setattr(value, "locator_kind", "utf8_bytes"),
                setattr(value, "locator_start_inclusive", "1"),
                setattr(value, "locator_end_exclusive", 2),
            ),
            "/locator/startInclusive",
        ),
    ],
)
def test_unknown_unions_and_invalid_numerics_fail_closed(
    mutate: Callable[[SimpleNamespace], None], field_path: str
) -> None:
    value = _native_resolved_fixture()
    mutate(value)
    with pytest.raises(fathomdb.EvidenceError) as raised:
        _map_native_resolved_evidence(value)
    assert raised.value.reason == "evidence_corrupt"
    assert raised.value.field_path == field_path
