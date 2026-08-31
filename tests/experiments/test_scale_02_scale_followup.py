"""Contract tests for the SCALE-02 post-boundary hypothesis runner."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import scale_02_scale_followup as followup


CONFIG = Path("experiments/configs/scale-02/scale-hypotheses.v1.json")


def test_config_freezes_two_hypotheses_and_three_points():
    config = followup.load_config(CONFIG)

    assert config["points"] == [25000, 40000, 50000]
    assert [item["id"] for item in config["treatments"]] == [
        "rank_mmap128",
        "rank_cache64",
    ]
    assert config["policy"]["steady_p50_ms_by_point"] == {
        "25000": 25,
        "40000": 40,
        "50000": 50,
    }
    assert config["authorization"]["ledger_ref"] == "seq-265"


def test_config_rejects_unknown_fields(tmp_path):
    document = json.loads(CONFIG.read_text(encoding="utf-8"))
    document["unexpected"] = True
    path = tmp_path / "config.json"
    path.write_text(json.dumps(document), encoding="utf-8")

    with pytest.raises(followup.Scale02ScaleFollowupError, match="keys drifted"):
        followup.load_config(path)


def test_cell_decision_requires_performance_and_exact_equivalence():
    policy = {
        "steady_p50_ms_by_point": {"40000": 40},
        "steady_p99_ms": 150,
        "max_rss_fraction": 0.8,
        "max_errors": 0,
        "max_timeouts": 0,
        "require_exact_ordered_top10": True,
    }
    cell = {
        "point": 40000,
        "complete_repetitions": True,
        "errors": 0,
        "timeouts": 0,
        "steady": {"p50": 30.0, "p99": 100.0},
        "upper_95": {"rss_fraction": 0.02},
        "equivalence": {"query_count": 100, "mismatch_count": 0},
    }

    assert followup.evaluate_cell(cell, policy)["eligible"] is True

    cell["equivalence"]["mismatch_count"] = 1
    decision = followup.evaluate_cell(cell, policy)
    assert decision["eligible"] is False
    assert decision["reasons"] == ["retrieval_equivalence"]


def test_committed_hypothesis_receipt_uses_logical_artifact_path():
    receipt = json.loads(
        Path(
            "experiments/runs/scale-02-scale-hypotheses-20260822T2328Z-55ce25d2/record.json"
        ).read_text(encoding="utf-8")
    )

    assert receipt["artifacts"] == [
        {
            "kind": "external_artifact_manifest",
            "path": "artifact-manifest.v1.json",
            "sha256": "96da584a4a49937f3baaccaed70d0207861584b957faf2e5b1e12c4ef76303ee",
        }
    ]
