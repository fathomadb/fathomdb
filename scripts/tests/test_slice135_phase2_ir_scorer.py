"""Human-labeled required-document retrieval scoring and fail-closed receipts."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


SCRIPTS = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "slice135_phase2_ir_scorer", SCRIPTS / "slice135_phase2_ir_scorer.py"
)
assert SPEC is not None and SPEC.loader is not None
SCORER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SCORER)


def _gold() -> list[dict[str, object]]:
    return [
        {
            "query_id": "enronqa:q1", "source": "enronqa", "query_class": "exact_fact",
            "expected_top_k_doc_ids": ["a"],
            "required_evidence": [{"doc_id": "a", "necessity": "required"}],
        },
        {
            "query_id": "enronqa:q2", "source": "enronqa", "query_class": "exact_fact",
            "expected_top_k_doc_ids": ["b"],
            "required_evidence": [{"doc_id": "b", "necessity": "required"}],
        },
        {
            "query_id": "qmsum:q3", "source": "qmsum", "query_class": "exploratory",
            "expected_top_k_doc_ids": ["c"],
            "required_evidence": [{"doc_id": "c", "necessity": "required"}],
        },
        {
            "query_id": "qaconv:q4", "source": "qaconv", "query_class": "negative",
            "expected_top_k_doc_ids": [], "required_evidence": [],
        },
    ]


def _observations() -> list[dict[str, object]]:
    return [
        {"query_id": "enronqa:q1", "status": "ok", "hits": [
            {"doc_id": "enronqa:a", "branch": "vector"},
        ]},
        {"query_id": "enronqa:q2", "status": "ok", "hits": [
            {"doc_id": "enronqa:x", "branch": "text"},
            {"doc_id": "enronqa:y", "branch": "vector"},
            {"doc_id": "enronqa:z", "branch": "text"},
            {"doc_id": "enronqa:b", "branch": "vector"},
        ]},
        {"query_id": "qmsum:q3", "status": "error", "error_type": "SearchError", "hits": []},
    ]


ALLOWED = {
    "enronqa:a", "enronqa:b", "enronqa:x", "enronqa:y", "enronqa:z", "qmsum:c",
}


def test_required_document_metrics_keep_errors_in_denominator() -> None:
    report = SCORER.score_version(_gold(), _observations(), ALLOWED)
    assert report["by_class"]["exact_fact"] == {
        "denominator": 2, "errors": 0, "recall_at_1": 0.5,
        "recall_at_5": 1.0, "recall_at_10": 1.0, "mrr_at_10": 0.625,
    }
    assert report["by_class"]["exploratory"] == {
        "denominator": 1, "errors": 1, "recall_at_1": 0.0,
        "recall_at_5": 0.0, "recall_at_10": 0.0, "mrr_at_10": 0.0,
    }
    assert report["negative_queries_omitted"] == 1
    assert report["per_query"]["enronqa:q2"]["required_rank"] == 4
    assert report["per_query"]["qmsum:q3"]["required_rank"] is None


def test_missing_query_is_rejected_instead_of_shrinking_denominator() -> None:
    with pytest.raises(ValueError, match="query"):
        SCORER.score_version(_gold(), _observations()[:-1], ALLOWED)


def test_unknown_hit_is_rejected_but_known_wrong_hit_is_a_miss() -> None:
    observations = _observations()
    observations[0]["hits"] = [{"doc_id": "enronqa:x", "branch": "text"}]
    report = SCORER.score_version(_gold(), observations, ALLOWED)
    assert report["per_query"]["enronqa:q1"]["required_rank"] is None
    observations[0]["hits"] = [{"doc_id": "enronqa:unknown", "branch": "text"}]
    with pytest.raises(ValueError, match="hit"):
        SCORER.score_version(_gold(), observations, ALLOWED)


def test_duplicate_hit_and_negative_query_observation_are_rejected() -> None:
    observations = _observations()
    observations[0]["hits"].append({"doc_id": "enronqa:a", "branch": "vector"})
    with pytest.raises(ValueError, match="duplicate"):
        SCORER.score_version(_gold(), observations, ALLOWED)
    observations = _observations()
    observations.append({"query_id": "qaconv:q4", "status": "ok", "hits": []})
    with pytest.raises(ValueError, match="query"):
        SCORER.score_version(_gold(), observations, ALLOWED)


def test_disagreeing_required_evidence_is_rejected() -> None:
    gold = _gold()
    gold[0]["required_evidence"] = [{"doc_id": "b", "necessity": "required"}]
    with pytest.raises(ValueError, match="required"):
        SCORER.score_version(gold, _observations(), ALLOWED)
