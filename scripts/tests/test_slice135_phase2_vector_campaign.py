"""Phase 2 fidelity campaign audit checks persisted inputs and complete pairing."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sqlite3

import fathomdb
import pytest


SCRIPTS = Path(__file__).resolve().parents[1]


def _module(name: str):
    path = SCRIPTS / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RUNNER = _module("slice135_phase2_vector_runner")
CAMPAIGN = _module("slice135_phase2_vector_campaign_audit")


def test_database_audit_detects_changed_canonical_body(tmp_path: Path) -> None:
    database = tmp_path / "fidelity.sqlite"
    before = tmp_path / "pre-isolation.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        engine.write([{"kind": "doc", "body": "orchard record", "logical_id": "A", "source_id": "source"}])
    finally:
        engine.close()
    RUNNER.isolate_lexical_index(database, before, expected_nodes=1)
    expected = [("A", "doc", "orchard record", "source")]
    assert CAMPAIGN.audit_databases(before, database, expected, vector_count=None)["canonical_nodes"] == 1
    with sqlite3.connect(database) as connection:
        connection.execute("UPDATE canonical_nodes SET body = 'wrong' WHERE logical_id = 'A'")
    with pytest.raises(ValueError):
        CAMPAIGN.audit_databases(before, database, expected, vector_count=None)


def test_paired_summary_requires_both_versions_and_same_query_set() -> None:
    baseline = {"query_count": 2, "k": 10, "per_query": {"q1": 0.8, "q2": 0.9}, "mean_recall": 0.85}
    candidate = {"query_count": 2, "k": 10, "per_query": {"q1": 0.9, "q2": 0.7}, "mean_recall": 0.8}
    paired = CAMPAIGN.paired_summary({"baseline": baseline, "candidate": candidate}, ["q1", "q2"])
    assert paired["mean_delta"] == pytest.approx(-0.05)
    assert paired["per_query_delta"] == {"q1": pytest.approx(0.1), "q2": pytest.approx(-0.2)}
    with pytest.raises(ValueError):
        CAMPAIGN.paired_summary({"baseline": baseline}, ["q1", "q2"])
    with pytest.raises(ValueError):
        CAMPAIGN.paired_summary({"baseline": baseline, "candidate": candidate}, ["q1", "wrong"])
