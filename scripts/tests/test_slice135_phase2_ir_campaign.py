"""Judged retrieval audit checks persisted mapping and complete pairing."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sqlite3

import pytest


PATH = Path(__file__).resolve().parents[1] / "slice135_phase2_ir_campaign_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_ir_campaign_audit", PATH)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


def test_persisted_canonical_body_change_is_rejected(tmp_path: Path) -> None:
    database = tmp_path / "retrieval.sqlite"
    with sqlite3.connect(database) as db:
        db.execute("CREATE TABLE canonical_nodes(logical_id TEXT, kind TEXT, body TEXT, source_id TEXT)")
        db.execute("INSERT INTO canonical_nodes VALUES('enronqa:a','doc','{\"summary\":\"Body A\"}','slice135-ir:enronqa')")
    expected = [("enronqa:a", "doc", '{"summary":"Body A"}', "slice135-ir:enronqa")]
    assert AUDIT.audit_database(database, expected, vector_count=None)["canonical_nodes"] == 1
    with sqlite3.connect(database) as db:
        db.execute("UPDATE canonical_nodes SET body='wrong'")
    with pytest.raises(ValueError, match="canonical"):
        AUDIT.audit_database(database, expected, vector_count=None)


def test_paired_summary_requires_both_complete_query_sets() -> None:
    baseline = {"by_class": {"exact_fact": {"recall_at_10": 0.5}},
                "per_query": {"q1": {"class": "exact_fact", "required_rank": 1},
                              "q2": {"class": "exact_fact", "required_rank": None}}}
    candidate = {"by_class": {"exact_fact": {"recall_at_10": 1.0}},
                 "per_query": {"q1": {"class": "exact_fact", "required_rank": 1},
                               "q2": {"class": "exact_fact", "required_rank": 5}}}
    paired = AUDIT.paired_summary({"baseline": baseline, "candidate": candidate}, ["q1", "q2"])
    assert paired["by_class"]["exact_fact"]["recall_at_10_delta"] == 0.5
    assert paired["candidate_better"] == 1
    with pytest.raises(ValueError, match="pair"):
        AUDIT.paired_summary({"baseline": baseline}, ["q1", "q2"])
    with pytest.raises(ValueError, match="query"):
        AUDIT.paired_summary({"baseline": baseline, "candidate": candidate}, ["q1", "wrong"])
