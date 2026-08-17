"""Review-remediation tests for qualified CORPUS-01 lifecycle evidence."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import corpus_matrix


ROOT = Path(__file__).resolve().parents[2]
MATRIX_PATH = ROOT / "experiments/configs/corpus-01/corpus-matrix.v1.json"


def _matrix() -> dict[str, object]:
    return json.loads(MATRIX_PATH.read_text(encoding="utf-8"))


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
        "preflight": {
            "payload_root_sha256": "b" * 64,
            "license_copy_sha256": "c" * 64,
        },
        "records": [],
    }

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="preflight keys"):
        corpus_matrix.validate_qualified_human_gold_manifest(incomplete, matrix)


def test_complete_portfolio_needs_qualified_evidence_for_each_required_category():
    matrix = _matrix()

    with pytest.raises(corpus_matrix.CorpusMatrixError, match="insufficient portfolio/category evidence"):
        corpus_matrix.validate_portfolio_coverage(matrix, qualified_evidence=())
