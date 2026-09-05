"""Slice 50 source-complete evidence through the installed Python surface."""

from __future__ import annotations

import hashlib

import fathomdb
import pytest  # pyright: ignore[reportMissingImports]


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
    engine.close()
