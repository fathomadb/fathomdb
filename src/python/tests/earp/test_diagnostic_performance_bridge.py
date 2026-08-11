"""The independent runner can execute a real diagnostic workload repeatedly."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.earp.config import resolve_config
from eval.performance.earp_adapter import (
    PerformancePlan,
    WorkloadRef,
    run_diagnostic_repetitions,
)

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_bridge_runs_real_fresh_store_and_warm_diagnostic_cells(tmp_path: Path) -> None:
    fixture = tmp_path / "fixture.jsonl"
    fixture.write_text(
        json.dumps(
            {
                "kind": "doc",
                "body": "the deal sheet is missing for March",
                "source_id": "source",
                "logical_id": "doc-a",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    config = {
        "schema_version": "earp.v1",
        "campaign": "diagnostic",
        "scenario": {
            "fixture": str(fixture),
            "query": {"call": "Engine.search_text_only", "text": "deal sheet", "limit": 10},
        },
    }
    resolution = resolve_config(config)
    assert resolution.scenario is not None, resolution.blockers
    workload = WorkloadRef(
        parent_run_id="quality-run",
        evidence_family_id="quality-run",
        config_sha256=resolution.scenario.config_sha256,
        candidate_sha="a" * 40,
        query_call=resolution.scenario.query_call,
        effective_knobs={"limit": 10},
    )

    samples = run_diagnostic_repetitions(
        workload=workload,
        plan=PerformancePlan(
            repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")
        ),
        scenario=resolution.scenario,
        config_doc=config,
        experiments_root=tmp_path / "scratch",
        experiment="earp-diagnostic-performance",
        ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
    )

    assert [(sample.treatment, sample.repetition) for sample in samples] == [
        ("fresh_store", 0),
        ("fresh_store_warm_query", 0),
    ]
    assert all({"open", "write", "query"} <= set(sample.phases_ms) for sample in samples)
    assert not (tmp_path / "scratch" / "runs").exists()


def test_bridge_refuses_to_mislabel_same_process_work_as_process_cold(
    tmp_path: Path,
) -> None:
    fixture = tmp_path / "fixture.jsonl"
    fixture.write_text(
        json.dumps(
            {"kind": "doc", "body": "deal", "source_id": "source", "logical_id": "doc-a"}
        )
        + "\n",
        encoding="utf-8",
    )
    config = {
        "schema_version": "earp.v1",
        "campaign": "diagnostic",
        "scenario": {
            "fixture": str(fixture),
            "query": {"call": "Engine.search_text_only", "text": "deal", "limit": 10},
        },
    }
    resolution = resolve_config(config)
    assert resolution.scenario is not None, resolution.blockers
    workload = WorkloadRef(
        parent_run_id="quality-run",
        evidence_family_id="quality-run",
        config_sha256=resolution.scenario.config_sha256,
        candidate_sha="a" * 40,
        query_call=resolution.scenario.query_call,
        effective_knobs={"limit": 10},
    )

    with pytest.raises(ValueError, match="fresh_store"):
        run_diagnostic_repetitions(
            workload=workload,
            plan=PerformancePlan(repetitions=1, treatments=("process_cold",)),
            scenario=resolution.scenario,
            config_doc=config,
            experiments_root=tmp_path / "scratch",
            experiment="earp-diagnostic-performance",
            ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
        )
