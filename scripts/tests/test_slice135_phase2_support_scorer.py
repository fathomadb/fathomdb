"""Support-set scoring requires every labeled item and every query."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


PATH = Path(__file__).resolve().parents[1] / "slice135_phase2_support_scorer.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_support_scorer", PATH)
assert SPEC is not None and SPEC.loader is not None
SCORER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SCORER)


def _inputs() -> tuple[list[dict], list[dict], set[str]]:
    queries = [
        {"query_id": "q1", "query_class": "two_hop", "text": "first?",
         "required_ids": ["a", "b"]},
        {"query_id": "q2", "query_class": "four_hop", "text": "second?",
         "required_ids": ["c", "d"], "strict_multi_session": True},
    ]
    observations = [
        {"query_id": "q1", "status": "ok", "hits": [
            {"doc_id": "a", "branch": "text"}, {"doc_id": "b", "branch": "vector"}]},
        {"query_id": "q2", "status": "ok", "hits": [
            {"doc_id": "c", "branch": "text"}, {"doc_id": "x", "branch": "vector"},
            {"doc_id": "d", "branch": "text"}]},
    ]
    return queries, observations, {"a", "b", "c", "d", "x"}


def test_complete_set_and_required_recall_at_10_and_20() -> None:
    queries, observations, allowed = _inputs()
    scored = SCORER.score_version(queries, observations, allowed, limit=20)
    assert scored["by_class"]["two_hop"]["complete_at_10"] == 1
    assert scored["by_class"]["four_hop"]["complete_at_10"] == 1
    assert scored["strict_multi_session"]["denominator"] == 1
    assert scored["per_query"]["q2"]["last_required_rank"] == 3


def test_known_wrong_hit_lowers_score() -> None:
    queries, observations, allowed = _inputs()
    observations[0]["hits"][1]["doc_id"] = "x"
    scored = SCORER.score_version(queries, observations, allowed, limit=20)
    assert scored["by_class"]["two_hop"]["complete_at_10"] == 0
    assert scored["by_class"]["two_hop"]["mean_support_recall_at_10"] == 0.5


@pytest.mark.parametrize("alter", [
    lambda rows: rows.pop(),
    lambda rows: rows.append(rows[0]),
    lambda rows: rows[0]["hits"].append({"doc_id": "unknown", "branch": "text"}),
    lambda rows: rows[0]["hits"].append(rows[0]["hits"][0]),
])
def test_missing_duplicate_or_unknown_receipts_rejected(alter) -> None:
    queries, observations, allowed = _inputs()
    alter(observations)
    with pytest.raises(ValueError):
        SCORER.score_version(queries, observations, allowed, limit=20)


def test_search_error_scores_zero_and_stays_in_denominator() -> None:
    queries, observations, allowed = _inputs()
    observations[0] = {"query_id": "q1", "status": "error", "error_type": "RuntimeError", "hits": []}
    scored = SCORER.score_version(queries, observations, allowed, limit=20)
    assert scored["by_class"]["two_hop"]["denominator"] == 1
    assert scored["by_class"]["two_hop"]["errors"] == 1
    assert scored["by_class"]["two_hop"]["complete_at_20"] == 0
