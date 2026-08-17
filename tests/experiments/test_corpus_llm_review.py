"""Human-intended guardrails for the authorized CORPUS-01 LLM review route."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import corpus_llm_review
from experiments import corpus_matrix


ROOT = Path(__file__).resolve().parents[2]
MATRIX_PATH = ROOT / "experiments/configs/corpus-01/corpus-matrix.v1.json"
PROTOCOL_PATH = ROOT / "experiments/configs/corpus-01/llm-review-protocol.v1.json"


def _matrix() -> dict[str, object]:
    return json.loads(MATRIX_PATH.read_text(encoding="utf-8"))


def _protocol() -> dict[str, object]:
    return json.loads(PROTOCOL_PATH.read_text(encoding="utf-8"))


def _manifest(matrix: dict[str, object], protocol: dict[str, object]) -> dict[str, object]:
    reviewers = ["a" * 64, "b" * 64]
    return {
        "schema_version": "corpus-01-llm-review-manifest.v1",
        "artifact_id": "corpus-01-llm-review-pilot-001",
        "scope": "partial_portfolio",
        "matrix_sha256": corpus_matrix.canonical_sha256(matrix),
        "protocol_sha256": corpus_matrix.canonical_sha256(protocol),
        "source_payload_sha256": "c" * 64,
        "license_copy_sha256": "d" * 64,
        "source_revision": "longmemeval-cleaned-98d7416c",
        "workset_sha256": "e" * 64,
        "reviewer_runs": [
            {
                "reviewer_id_sha256": reviewers[0],
                "provider": "openai",
                "model": "gpt-sol-high",
                "prompt_sha256": "f" * 64,
                "transcript_sha256": "1" * 64,
                "run_receipt_sha256": "2" * 64,
                "blinded_assignment_sha256": "3" * 64,
            },
            {
                "reviewer_id_sha256": reviewers[1],
                "provider": "anthropic",
                "model": "opus-5-high",
                "prompt_sha256": "4" * 64,
                "transcript_sha256": "5" * 64,
                "run_receipt_sha256": "6" * 64,
                "blinded_assignment_sha256": "7" * 64,
            },
        ],
        "records": [
            {
                "annotation_id": "corpus01-knowledge-update-01",
                "corpus_id": "longmemeval",
                "category": "knowledge_update",
                "source_locator_sha256": "8" * 64,
                "evidence_locator_sha256": "9" * 64,
                "reviewer_judgments": {reviewers[0]: "supported", reviewers[1]: "supported"},
                "judgment": "supported",
                "adjudication": "not_required",
            }
        ],
        "total_cost_usd": 0.0,
        "conclusion": "evidence_limited",
    }


def test_protocol_is_distinct_from_human_gold_and_binds_the_matrix():
    protocol = corpus_llm_review.validate_protocol(_protocol(), _matrix())

    assert protocol.reviewer_count == 2
    assert protocol.max_total_cost_usd == 20.0


def test_manifest_requires_two_distinct_model_identities_and_content_free_bindings():
    matrix = _matrix()
    protocol_document = _protocol()

    manifest = corpus_llm_review.validate_manifest(_manifest(matrix, protocol_document), matrix, protocol_document)

    assert manifest.categories == ("knowledge_update",)
    assert manifest.evidence_mode == "qualified_llm_review"


def test_manifest_fails_closed_when_reviewer_model_identity_is_reused():
    matrix = _matrix()
    protocol = _protocol()
    document = _manifest(matrix, protocol)
    document["reviewer_runs"][1]["provider"] = "openai"
    document["reviewer_runs"][1]["model"] = "gpt-sol-high"

    with pytest.raises(corpus_llm_review.LlmReviewError, match="independent model identities"):
        corpus_llm_review.validate_manifest(document, matrix, protocol)


def test_manifest_rejects_raw_fields_and_cost_over_cap():
    matrix = _matrix()
    protocol = _protocol()
    raw = _manifest(matrix, protocol)
    raw["records"][0]["question"] = "must remain external"
    with pytest.raises(corpus_llm_review.LlmReviewError, match="record keys"):
        corpus_llm_review.validate_manifest(raw, matrix, protocol)

    expensive = _manifest(matrix, protocol)
    expensive["total_cost_usd"] = 20.01
    with pytest.raises(corpus_llm_review.LlmReviewError, match="cost exceeds"):
        corpus_llm_review.validate_manifest(expensive, matrix, protocol)
