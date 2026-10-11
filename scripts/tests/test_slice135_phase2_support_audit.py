"""Paired support audit preserves case completeness and loss visibility."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


PATH = Path(__file__).resolve().parents[1] / "slice135_phase2_support_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_support_audit", PATH)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


def _scores() -> dict:
    return {
        "baseline": {"per_query": {
            "a": {"class": "two_hop", "complete_at_10": True, "complete_at_20": True},
            "b": {"class": "two_hop", "complete_at_10": False, "complete_at_20": True},
        }},
        "candidate": {"per_query": {
            "a": {"class": "two_hop", "complete_at_10": False, "complete_at_20": True},
            "b": {"class": "two_hop", "complete_at_10": True, "complete_at_20": True},
        }},
    }


def test_pair_reports_case_losses_and_wins() -> None:
    pair = AUDIT.paired_summary(_scores(), ["a", "b"])
    assert pair["candidate_worse_at_10"] == ["a"]
    assert pair["candidate_better_at_10"] == ["b"]
    assert pair["candidate_worse_at_20"] == []


def test_incomplete_pair_rejected() -> None:
    scores = _scores()
    del scores["candidate"]["per_query"]["b"]
    with pytest.raises(ValueError):
        AUDIT.paired_summary(scores, ["a", "b"])
