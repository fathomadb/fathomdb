"""Contract tests for the generated capability experiment scoreboard."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import scorecard


def _record(*, run_id: str, metrics: dict) -> dict:
    return {
        "schema_version": "experiments.record.v1",
        "run_id": run_id,
        "experiment": "locomo-capability",
        "title": "LOCOMO capability",
        "verdict": "complete",
        "read": "safe aggregate receipt",
        "code": {"git_sha": "abc", "dirty": False, "branch": "campaign", "baseline_commit": None},
        "config": {"sha256": "deadbeef", "path": None, "resolved": {}},
        "corpus": {"source": "LOCOMO", "manifest_sha256": "hash", "datasets": []},
        "seeds": {},
        "env": {"python": "3.12", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
        "metrics": metrics,
        "cost_usd": 0.0,
        "tdd_evidence": {},
        "tests": None,
        "files_changed": [],
        "artifacts": [],
        "review": None,
        "open_questions": [],
    }


def _write_record(root: Path, record: dict) -> None:
    path = root / "runs" / record["run_id"] / "record.json"
    path.parent.mkdir(parents=True)
    path.write_text(json.dumps(record), encoding="utf-8")


def _metrics() -> dict:
    return {
        "capability_scorecard": {
            "configuration": "a0-turn-fts",
            "m1_r_at_10": 0.66,
            "m2_mrr": 0.41,
            "m2_r_at_1": 0.31,
            "m2_ndcg_at_10": 0.48,
            "m4_proxy_temporal_evidence_recall": 0.52,
            "m6_query_p95_ms": 12.5,
            "m7_ready_to_search_ms": 44.0,
        }
    }


def test_scorecard_lists_only_capability_receipts_and_is_deterministic(tmp_path):
    _write_record(tmp_path, _record(run_id="z-run", metrics=_metrics()))
    _write_record(tmp_path, _record(run_id="legacy-run", metrics={"phase": "legacy"}))

    rows = scorecard.scorecard_rows(tmp_path / "runs")
    assert rows == [{"run_id": "z-run", "verdict": "complete", **_metrics()["capability_scorecard"]}]

    output = tmp_path / "SCOREBOARD.md"
    scorecard.regen_scoreboard(runs_dir=tmp_path / "runs", output_path=output)
    once = output.read_text(encoding="utf-8")
    scorecard.regen_scoreboard(runs_dir=tmp_path / "runs", output_path=output)
    assert output.read_text(encoding="utf-8") == once
    assert "a0-turn-fts" in once
    assert "legacy-run" not in once


def test_scorecard_rejects_unknown_or_non_numeric_metric_fields(tmp_path):
    metrics = _metrics()
    metrics["capability_scorecard"]["raw_output_path"] = "/srv/secret/raw.json"
    _write_record(tmp_path, _record(run_id="unsafe", metrics=metrics))

    with pytest.raises(ValueError, match="keys mismatch"):
        scorecard.scorecard_rows(tmp_path / "runs")

    metrics = _metrics()
    metrics["capability_scorecard"]["m1_r_at_10"] = "0.66"
    _write_record(tmp_path, _record(run_id="bad-number", metrics=metrics))

    with pytest.raises(ValueError, match="finite number"):
        scorecard.scorecard_rows(tmp_path / "runs")
