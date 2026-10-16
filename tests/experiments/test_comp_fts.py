"""Measurement-correctness checks for the COMP side-plan FTS harness."""

from __future__ import annotations

import json
import math
from pathlib import Path

import pytest

from experiments import comp_fts


def test_roster_balances_arm_order_across_four_repetitions():
    assert comp_fts.roster(("a", "b")) == [
        "r1-a", "r1-b", "r2-b", "r2-a", "r3-b", "r3-a", "r4-a", "r4-b",
    ]


def test_pending_skips_recorded_cells(tmp_path: Path):
    (tmp_path / "cells").mkdir()
    (tmp_path / "cells" / "r1-a.json").write_text("{}")
    assert comp_fts.pending(["r1-a", "r1-b"], tmp_path) == ["r1-b"]


def test_score_query_matches_hand_computed_values():
    judgments = {"d1": 2, "d2": 1, "d3": 0, "d4": 1}
    scores = comp_fts.score_query(["x", "d2", "d1"], judgments)
    dcg = 1 / math.log2(3) + 2 / math.log2(4)
    idcg = 2 / math.log2(2) + 1 / math.log2(3) + 1 / math.log2(4)
    assert scores["ndcg_at_10"] == pytest.approx(dcg / idcg)
    assert scores["recall_at_10"] == pytest.approx(2 / 3)
    assert scores["mrr_at_10"] == pytest.approx(1 / 2)


def test_score_query_ignores_hits_beyond_cutoff():
    scores = comp_fts.score_query([f"x{i}" for i in range(10)] + ["d1"], {"d1": 1})
    assert scores == {"ndcg_at_10": 0.0, "recall_at_10": 0.0, "mrr_at_10": 0.0}


def _available(arm: str) -> None:
    if arm == "surrealdb":
        if not comp_fts.SURREAL_DRIVER.exists():
            pytest.skip("surrealdb driver not built")
    else:
        pytest.importorskip(arm)


def _inputs(tmp_path: Path) -> Path:
    inputs = tmp_path / "in"
    inputs.mkdir()
    docs = [
        {"id": "d1", "body": "zebra migration across the savanna", "cohort": "c01"},
        {"id": "d2", "body": "penguin colonies on antarctic ice", "cohort": "c10"},
        {"id": "d3", "body": "volcanic soil and coffee farming", "cohort": "c33"},
        {"id": "d4", "body": "penguin chicks huddle on ice", "cohort": "c33"},
    ]
    (inputs / "corpus.json").write_text(json.dumps(docs))
    (inputs / "queries.json").write_text(json.dumps([
        {"id": "q1", "text": "penguin colonies"}, {"id": "q2", "text": "volcanic coffee"},
    ]))
    (inputs / "probes.json").write_text(json.dumps([
        {"position": 0, "id": "d1", "text": "zebra"}, {"position": 3, "id": "d4", "text": "chicks"},
    ]))
    return inputs


@pytest.mark.parametrize("arm", ["fathomdb", "lancedb", "surrealdb"])
def test_real_cell_returns_expected_document_first(arm: str, tmp_path: Path):
    _available(arm)
    inputs = _inputs(tmp_path)
    cell = comp_fts.run_cell(arm, 1, inputs, tmp_path / "work")
    top = {r["query_id"]: r["ids"][0] for r in cell["results"]}
    assert top == {"q1": "d2", "q2": "d3"}
    assert all(r["latency_ms"] > 0 for r in cell["results"])
    assert cell["disk_bytes"] > 0 and not (tmp_path / "work").exists()


@pytest.mark.parametrize("arm", ["fathomdb", "surrealdb"])
def test_real_scale_cell_reaches_ready_and_survives_reopen(arm: str, tmp_path: Path):
    _available(arm)
    cell = comp_fts.run_scale_cell(arm, 1, _inputs(tmp_path), tmp_path / "work", batch=2)
    assert cell["query_ready_s"] is not None and cell["probe_failing"] == []
    assert cell["sentinel_found_after_reopen"]
    assert len(cell["cold"]) == 2 and len(cell["steady"]) == 20
    assert cell["ingest_ack_s"] <= cell["build_return_s"] <= cell["query_ready_s"]


@pytest.mark.parametrize("arm", ["fathomdb", "surrealdb"])
def test_real_filtered_cell_returns_only_the_requested_cohort(arm: str, tmp_path: Path):
    _available(arm)
    cell = comp_fts.run_filtered_cell(arm, 1, _inputs(tmp_path), tmp_path / "work")
    c33 = cell["passes"]["c33"]
    assert c33["violations"] == 0
    top = {r["query_id"]: r["ids"] for r in c33["results"]}
    assert top["q1"] == ["d4"] and top["q2"] == ["d3"]
    assert {r["query_id"]: r["ids"] for r in cell["passes"]["c10"]["results"]} == {"q1": ["d2"], "q2": []}
