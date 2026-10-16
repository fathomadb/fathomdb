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


@pytest.mark.parametrize("arm", ["fathomdb", "lancedb"])
def test_real_cell_returns_expected_document_first(arm: str, tmp_path: Path):
    pytest.importorskip(arm)
    inputs = tmp_path / "in"
    inputs.mkdir()
    docs = [
        {"id": "d1", "body": "zebra migration across the savanna"},
        {"id": "d2", "body": "penguin colonies on antarctic ice"},
        {"id": "d3", "body": "volcanic soil and coffee farming"},
    ]
    (inputs / "corpus.json").write_text(json.dumps(docs))
    (inputs / "queries.json").write_text(json.dumps([
        {"id": "q1", "text": "penguin colonies"}, {"id": "q2", "text": "volcanic coffee"},
    ]))
    cell = comp_fts.run_cell(arm, 1, inputs, tmp_path / "work")
    top = {r["query_id"]: r["ids"][0] for r in cell["results"]}
    assert top == {"q1": "d2", "q2": "d3"}
    assert all(r["latency_ms"] > 0 for r in cell["results"])
    assert cell["disk_bytes"] > 0 and not (tmp_path / "work").exists()
