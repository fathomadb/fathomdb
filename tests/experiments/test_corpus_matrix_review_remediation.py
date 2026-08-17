"""Review-remediation tests for qualified CORPUS-01 lifecycle evidence."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import corpus_matrix


ROOT = Path(__file__).resolve().parents[2]
MATRIX_PATH = ROOT / "experiments/configs/corpus-01/corpus-matrix.v1.json"
PROTOCOL_PATH = ROOT / "experiments/configs/corpus-01/human-gold-protocol.v2.json"


def _matrix() -> dict[str, object]:
    return json.loads(MATRIX_PATH.read_text(encoding="utf-8"))


def _protocol() -> dict[str, object]:
    return json.loads(PROTOCOL_PATH.read_text(encoding="utf-8"))


def _qualified_manifest(matrix: dict[str, object], protocol: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": "corpus-01-human-gold-manifest.v2",
        "artifact_id": "corpus-01-lifecycle-pilot-001",
        "scope": "pilot",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix),
        "protocol_sha256": corpus_matrix.canonical_sha256(protocol),
        "preflights": [{
            "corpus_id": "locomo", "category": "knowledge_update",
            "source_payload_sha256": "a" * 64, "license_copy_sha256": "b" * 64,
            "source_revision": "locomo-main-pinned", "selected_class_counts": [{"class": "factoid", "count": 20}],
            "exclusions_sha256": "c" * 64, "metric": "retrieval_recall_at_k",
            "paired_power_sha256": "d" * 64, "claim_id": "knowledge-update-locomo-pilot",
            "claim_sha256": "e" * 64,
        }],
        "records": [{
            "annotation_id": "annotation-001", "corpus_id": "locomo", "category": "knowledge_update",
            "source_locator_sha256": "f" * 64, "evidence_locator_sha256": "1" * 64,
            "judgment": "supported", "reviewer_hashes": ["2" * 64, "3" * 64],
            "adjudication": "not_required",
        }],
    }


def _approved_registry(amendment: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": "corpus-01-approved-amendment-registry.v1",
        "registry_id": "corpus-01-approved-amendments",
        "entries": [{
            "amendment_sha256": corpus_matrix.canonical_sha256(amendment),
            "corpus_id": amendment["corpus_id"],
            "category": amendment["category"],
            "approval_ref": amendment["approval_ref"],
        }],
    }


def test_native_and_human_gold_eligibility_are_explicitly_distinct():
    matrix = _matrix()

    native = corpus_matrix.validate_evaluation_eligibility(
        matrix, corpus_id="locomo", category="time_scoped_validity"
    )

    assert native.evidence_mode == "native_corpus"
    with pytest.raises(corpus_matrix.CorpusMatrixError, match="unsupported without approved human-gold amendment"):
        corpus_matrix.validate_evaluation_eligibility(
            matrix, corpus_id="locomo", category="knowledge_update"
        )


def test_qualified_human_gold_requires_all_factual_preflight_bindings():
    matrix = _matrix()
    incomplete = {
        "schema_version": "corpus-01-human-gold-manifest.v2",
        "artifact_id": "corpus-01-lifecycle-pilot-001",
        "scope": "pilot",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix),
        "protocol_sha256": "a" * 64,
        "preflights": [{
            "source_payload_sha256": "b" * 64,
            "license_copy_sha256": "c" * 64,
        }],
        "records": [],
    }

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="preflight keys"):
        corpus_matrix.validate_qualified_human_gold_manifest(incomplete, matrix)


def test_approved_amendment_is_the_only_route_that_qualifies_an_unsupported_pair():
    matrix = _matrix()
    protocol = _protocol()
    manifest_document = _qualified_manifest(matrix, protocol)

    qualified = corpus_matrix.validate_qualified_human_gold_manifest(
        manifest_document, matrix, protocol
    )
    amendment_document = {
        "schema_version": "corpus-01-human-gold-amendment.v1",
        "amendment_id": "corpus-01-locomo-knowledge-update-001",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix),
        "qualified_manifest_sha256": qualified.manifest_sha256,
        "corpus_id": "locomo", "category": "knowledge_update",
        "approval_ref": "seq-249", "eligibility": "approved_human_gold",
    }
    amendment = corpus_matrix.validate_human_gold_amendment(
        amendment_document, matrix, qualified, approved_registry=_approved_registry(amendment_document)
    )

    eligibility = corpus_matrix.validate_evaluation_eligibility(
        matrix, corpus_id="locomo", category="knowledge_update", amendment=amendment
    )

    assert eligibility.evidence_mode == "qualified_human_gold"


def test_fabricated_ledger_sequence_cannot_approve_an_unsupported_pair():
    matrix = _matrix()
    protocol = _protocol()
    qualified = corpus_matrix.validate_qualified_human_gold_manifest(
        _qualified_manifest(matrix, protocol), matrix, protocol
    )
    fabricated_amendment = {
        "schema_version": "corpus-01-human-gold-amendment.v1",
        "amendment_id": "corpus-01-locomo-knowledge-update-fabricated",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix),
        "qualified_manifest_sha256": qualified.manifest_sha256,
        "corpus_id": "locomo", "category": "knowledge_update",
        "approval_ref": "seq-999999", "eligibility": "approved_human_gold",
    }
    empty_registry = {
        "schema_version": "corpus-01-approved-amendment-registry.v1",
        "registry_id": "corpus-01-approved-amendments",
        "entries": [],
    }

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="approved-amendment registry"):
        corpus_matrix.validate_human_gold_amendment(
            fabricated_amendment, matrix, qualified, approved_registry=empty_registry
        )


def test_preflight_rejects_an_undeclared_class_or_metric_before_human_evidence_counts():
    matrix = _matrix()
    protocol = _protocol()
    document = _qualified_manifest(matrix, protocol)
    document["preflights"][0]["selected_class_counts"][0]["class"] = "invented_class"

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="selected class"):
        corpus_matrix.validate_qualified_human_gold_manifest(document, matrix, protocol)


def test_qualified_manifest_rejects_raw_fields_before_any_human_gold_can_count():
    matrix = _matrix()
    protocol = _protocol()
    document = _qualified_manifest(matrix, protocol)
    document["records"][0]["question"] = "must not be committed"

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="qualified record keys"):
        corpus_matrix.validate_qualified_human_gold_manifest(document, matrix, protocol)


def test_complete_portfolio_needs_qualified_evidence_for_each_required_category():
    matrix = _matrix()

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="insufficient portfolio/category evidence"):
        corpus_matrix.validate_portfolio_coverage(matrix, qualified_evidence=())
