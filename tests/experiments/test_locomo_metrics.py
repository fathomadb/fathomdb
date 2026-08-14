"""Tests for safe retrieval metrics over LOCOMO prediction sidecars."""

from __future__ import annotations

import json
from pathlib import Path

from experiments.locomo_metrics import summarize_predictions
from experiments.locomo_provenance import search_request_fingerprint


def _write_prediction(root: Path, name: str, *, user_id: str, question: str, evidence: list[str], category: int) -> None:
    (root / name).write_text(json.dumps({
        "question_id": name.removesuffix(".json"), "user_id": user_id, "question": question,
        "evidence": evidence, "category": category,
    }), encoding="utf-8")


def test_safe_metric_summary_uses_hashed_queries_and_reports_temporal_proxy(tmp_path):
    _write_prediction(tmp_path, "conv0_q0.json", user_id="locomo_0_run", question="alpha", evidence=["D1:1"], category=2)
    _write_prediction(tmp_path, "conv0_q1.json", user_id="locomo_0_run", question="beta", evidence=["D1:2", "D1:3"], category=1)
    _write_prediction(tmp_path, "conv0_q2.json", user_id="locomo_0_run", question="unscored", evidence=[], category=3)
    sidecar = {
        "schema_version": "locomo-facade-provenance.v1",
        "requests": {
            search_request_fingerprint("locomo_0_run", "alpha"): [
                {"conversation_id": "locomo-0", "session_id": "session-1", "turn_ids": ["D1:1"]},
            ],
            search_request_fingerprint("locomo_0_run", "beta"): [
                {"conversation_id": "locomo-0", "session_id": "session-1", "turn_ids": ["D1:9"]},
                {"conversation_id": "locomo-0", "session_id": "session-1", "turn_ids": ["D1:2"]},
            ],
        },
    }

    summary = summarize_predictions(tmp_path, sidecar)

    assert summary["aggregate"] == {
        "n": 2, "excluded_no_evidence": 1, "r_at_5": 1.0, "r_at_10": 1.0, "r_at_20": 1.0,
        "mrr": 0.75, "r_at_1": 0.5, "ndcg_at_10": 0.6934264036172708,
        "temporal_evidence_recall": 1.0,
    }
    assert summary["per_question"] == [
        {"question_id": "conv0_q0", "category": 2, "r_at_10": 1.0},
        {"question_id": "conv0_q1", "category": 1, "r_at_10": 1.0},
    ]
    assert "alpha" not in json.dumps(summary)
    assert "D1:1" not in json.dumps(summary)
