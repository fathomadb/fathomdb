"""Phase 2 graph, evidence and erasure gold is checked on a real database."""

from __future__ import annotations

from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import sqlite3

import fathomdb
import pytest


SCRIPTS = Path(__file__).resolve().parents[1]
FIXTURE = (
    SCRIPTS.parent
    / "dev/plans/0.8.27/features/slice-135/phase2-graph-evidence-fixture.json"
)


def _module(name: str):
    path = SCRIPTS / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RUNNER = _module("slice135_phase2_graph_evidence_runner")
AUDITOR = _module("slice135_phase2_graph_evidence_audit")


def _fixture() -> dict:
    return json.loads(FIXTURE.read_text())


def _observations() -> dict:
    return {key: deepcopy(value) for key, value in _fixture()["gold"].items() if key != "persisted"}


def test_graph_evidence_erasure_and_reopen_against_real_database(tmp_path: Path) -> None:
    fixture = _fixture()
    database = tmp_path / "graph-evidence.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        engine.write(fixture["writes"])
        engine.drain(timeout_s=30)
        assert RUNNER.observe_before(engine, fixture) == fixture["gold"]["before"]
        report = RUNNER.erase(engine, fixture)
        assert report == fixture["gold"]["erase"]
        assert RUNNER.observe_after(engine, fixture) == fixture["gold"]["after_erase"]
    finally:
        engine.close()
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        assert RUNNER.observe_after(engine, fixture) == fixture["gold"]["after_reopen"]
    finally:
        engine.close()
    assert AUDITOR.audit_database(database, fixture) == {
        "canonical_nodes": 5,
        "canonical_edges": 3,
        "evidence_fts_residue": 0,
        "integrity": "ok",
    }
    with sqlite3.connect(database) as connection:
        connection.execute("UPDATE canonical_nodes SET body = 'tampered' WHERE logical_id = 'R'")
    with pytest.raises(ValueError):
        AUDITOR.audit_database(database, fixture)


def test_auditor_accepts_complete_predeclared_observations() -> None:
    assert AUDITOR.audit_observations(_observations(), _fixture()) == {
        "phases": 4,
        "graph_depth1_nodes": 2,
        "graph_depth2_nodes": 3,
        "evidence_refs": 1,
        "erased_nodes": 2,
    }


@pytest.mark.parametrize(
    ("phase", "field", "replacement"),
    [
        ("before", "graph_depth1", []),
        ("before", "resolved", {"canonical_source_body": "wrong"}),
        ("erase", "nodes_excised", 1),
        ("after_reopen", "evidence_hits", [{"body": "stale"}]),
    ],
)
def test_auditor_rejects_wrong_graph_evidence_erasure_or_reopen(
    phase: str, field: str, replacement: object,
) -> None:
    observed = _observations()
    observed[phase][field] = replacement
    with pytest.raises(ValueError):
        AUDITOR.audit_observations(observed, _fixture())


def test_auditor_rejects_missing_phase() -> None:
    observed = _observations()
    del observed["after_erase"]
    with pytest.raises(ValueError):
        AUDITOR.audit_observations(observed, _fixture())
