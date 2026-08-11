"""Derive a performance workload from durable EARP evidence, never a duplicate CLI."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from eval.performance.earp_adapter import load_earp_workload


def test_adapter_derives_exact_workload_from_quality_artifacts(tmp_path: Path) -> None:
    run = tmp_path / "experiments" / "runs" / "quality-run"
    run.mkdir(parents=True)
    (run / "record.json").write_text(
        json.dumps({"code": {"git_sha": "b" * 40}}), encoding="utf-8"
    )
    (run / "earp.result.v1.json").write_text(
        json.dumps(
            {
                "scenario": {
                    "config_sha256": "a" * 64,
                    "query_call": "Engine.search",
                    "fanout_used": 10,
                },
                "effective_knobs": {"rerank_depth": 20, "alpha": 0.7},
            }
        ),
        encoding="utf-8",
    )

    workload = load_earp_workload(tmp_path / "experiments", "quality-run")

    assert workload.parent_run_id == "quality-run"
    assert workload.evidence_family_id == "quality-run"
    assert workload.config_sha256 == "a" * 64
    assert workload.candidate_sha == "b" * 40
    assert workload.effective_knobs == {"rerank_depth": 20, "alpha": 0.7, "limit": 10}


def test_adapter_refuses_quality_artifacts_without_candidate_provenance(tmp_path: Path) -> None:
    run = tmp_path / "experiments" / "runs" / "quality-run"
    run.mkdir(parents=True)
    (run / "record.json").write_text(json.dumps({"code": {"git_sha": ""}}), encoding="utf-8")
    (run / "earp.result.v1.json").write_text(
        json.dumps(
            {
                "scenario": {
                    "config_sha256": "a" * 64,
                    "query_call": "Engine.search_text_only",
                }
            }
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="candidate SHA"):
        load_earp_workload(tmp_path / "experiments", "quality-run")
