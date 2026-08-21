"""Human-intended guardrails for the CORPUS-01 portfolio and gold protocol."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import corpus_matrix


ROOT = Path(__file__).resolve().parents[2]
MATRIX_PATH = ROOT / "experiments/configs/corpus-01/corpus-matrix.v1.json"
PROTOCOL_PATH = ROOT / "experiments/configs/corpus-01/human-gold-protocol.v1.json"


def _load(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def test_matrix_covers_every_required_portfolio_family_and_lifecycle_gap():
    matrix = corpus_matrix.validate_matrix(_load(MATRIX_PATH))

    assert matrix.corpus_ids == (
        "locomo",
        "longmemeval",
        "timelineqa",
        "timeqa",
        "test_of_time",
        "ir_c",
        "beir",
        "musique",
        "summhay",
        "apnews_benchmarkqed",
    )
    assert matrix.human_gold_categories == (
        "knowledge_update",
        "supersession",
        "source_erasure",
        "time_scoped_validity",
    )
    assert "knowledge_update" not in matrix.entry("locomo").supported_categories
    assert matrix.entry("locomo").payload_state == "external_payload_not_verified"
    assert matrix.entry("longmemeval").payload_state == "not_acquired"
    assert matrix.entry("ir_c").metrics == ("evidence_recall_at_k", "mrr", "ndcg")
    assert matrix.entry("musique").metrics == ("supporting_evidence_recall_at_k", "answer_f1")


def test_matrix_rejects_a_license_or_payload_assertion_without_a_factual_basis():
    document = _load(MATRIX_PATH)
    document["corpora"][0]["license_basis"] = "unverified"

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="license_basis"):
        corpus_matrix.validate_matrix(document)


def test_protocol_requires_external_only_human_records_and_all_lifecycle_categories():
    matrix_document = _load(MATRIX_PATH)
    protocol_document = _load(PROTOCOL_PATH)

    protocol = corpus_matrix.validate_human_gold_protocol(protocol_document, matrix_document)

    assert protocol.matrix_sha256 == corpus_matrix.canonical_sha256(matrix_document)
    assert protocol.required_reviewer_count == 2
    assert protocol.prohibited_fields == (
        "answer",
        "answer_text",
        "evidence_text",
        "question",
        "raw_payload",
        "verbatim_quote",
    )


def test_external_gold_manifest_is_opaque_double_reviewed_and_bound_to_protocol():
    matrix_document = _load(MATRIX_PATH)
    protocol_document = _load(PROTOCOL_PATH)
    protocol = corpus_matrix.validate_human_gold_protocol(protocol_document, matrix_document)
    document = {
        "schema_version": "corpus-01-human-gold-manifest.v1",
        "artifact_id": "corpus-01-lifecycle-pilot-001",
        "scope": "pilot",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix_document),
        "protocol_sha256": corpus_matrix.canonical_sha256(protocol_document),
        "payload_root_sha256": "a" * 64,
        "records": [
            {
                "annotation_id": "annotation-001",
                "corpus_id": "locomo",
                "category": "time_scoped_validity",
                "source_locator_sha256": "b" * 64,
                "evidence_locator_sha256": "c" * 64,
                "judgment": "supported",
                "reviewer_hashes": ["d" * 64, "e" * 64],
                "adjudication": "not_required",
            }
        ],
    }

    manifest = corpus_matrix.validate_external_gold_manifest(document, protocol)

    assert manifest.record_count == 1
    assert manifest.categories == ("time_scoped_validity",)
    assert "locomo" in manifest.corpus_ids
    assert "answer" not in json.dumps(document)


def test_complete_manifest_requires_all_lifecycle_categories_and_known_corpus():
    matrix_document = _load(MATRIX_PATH)
    protocol = corpus_matrix.validate_human_gold_protocol(_load(PROTOCOL_PATH), matrix_document)
    document = {
        "schema_version": "corpus-01-human-gold-manifest.v1",
        "artifact_id": "corpus-01-lifecycle-complete-001", "scope": "portfolio_complete",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix_document),
        "protocol_sha256": protocol.protocol_sha256, "payload_root_sha256": "a" * 64,
        "records": [{
            "annotation_id": "annotation-001", "corpus_id": "unregistered_corpus",
            "category": "time_scoped_validity", "source_locator_sha256": "b" * 64,
            "evidence_locator_sha256": "c" * 64, "judgment": "supported",
            "reviewer_hashes": ["d" * 64, "e" * 64], "adjudication": "not_required",
        }],
    }

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="known matrix corpus"):
        corpus_matrix.validate_external_gold_manifest(document, protocol)


@pytest.mark.parametrize("field", ["question", "answer_text", "evidence_text", "raw_payload"])
def test_external_gold_manifest_rejects_raw_or_verbatim_payload_fields(field):
    matrix_document = _load(MATRIX_PATH)
    protocol = corpus_matrix.validate_human_gold_protocol(_load(PROTOCOL_PATH), matrix_document)
    document = {
        "schema_version": "corpus-01-human-gold-manifest.v1",
        "artifact_id": "corpus-01-lifecycle-pilot-001",
        "scope": "pilot",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix_document),
        "protocol_sha256": protocol.protocol_sha256,
        "payload_root_sha256": "a" * 64,
        "records": [{
            "annotation_id": "annotation-001", "corpus_id": "locomo",
            "category": "time_scoped_validity", "source_locator_sha256": "b" * 64,
            "evidence_locator_sha256": "c" * 64, "judgment": "supported",
            "reviewer_hashes": ["d" * 64, "e" * 64], "adjudication": "not_required",
        }],
    }
    document["records"][0][field] = "must never enter the repository"

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="record keys"):
        corpus_matrix.validate_external_gold_manifest(document, protocol)


def test_external_gold_manifest_fails_closed_when_a_required_reviewer_is_missing():
    matrix_document = _load(MATRIX_PATH)
    protocol = corpus_matrix.validate_human_gold_protocol(_load(PROTOCOL_PATH), matrix_document)
    document = {
        "schema_version": "corpus-01-human-gold-manifest.v1",
        "artifact_id": "corpus-01-lifecycle-pilot-001",
        "scope": "pilot",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix_document),
        "protocol_sha256": protocol.protocol_sha256,
        "payload_root_sha256": "a" * 64,
        "records": [{
            "annotation_id": "annotation-001", "corpus_id": "locomo",
            "category": "time_scoped_validity", "source_locator_sha256": "b" * 64,
            "evidence_locator_sha256": "c" * 64, "judgment": "supported",
            "reviewer_hashes": ["d" * 64], "adjudication": "not_required",
        }],
    }

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="reviewer_hashes"):
        corpus_matrix.validate_external_gold_manifest(document, protocol)
