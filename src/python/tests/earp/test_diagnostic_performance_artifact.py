"""A diagnostic performance execution publishes only the independent artifact."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.earp.config import resolve_config
from eval.performance.earp_adapter import (
    PERFORMANCE_RESULT_NAME,
    PerformancePlan,
    WorkloadRef,
    run_and_write_diagnostic_performance,
)

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_diagnostic_performance_writes_linked_independent_artifact(tmp_path: Path) -> None:
    fixture = tmp_path / "fixture.jsonl"
    fixture.write_text(
        json.dumps(
            {
                "kind": "doc",
                "body": "deal sheet",
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
            "query": {"call": "Engine.search_text_only", "text": "deal", "limit": 10},
        },
    }
    resolution = resolve_config(config)
    assert resolution.scenario is not None, resolution.blockers
    outcome = run_and_write_diagnostic_performance(
        workload=WorkloadRef(
            parent_run_id="quality-run",
            evidence_family_id="quality-run",
            config_sha256=resolution.scenario.config_sha256,
            candidate_sha="a" * 40,
            query_call=resolution.scenario.query_call,
            effective_knobs={"limit": 10},
        ),
        plan=PerformancePlan(repetitions=1, treatments=("fresh_store", "warm")),
        scenario=resolution.scenario,
        config_doc=config,
        experiments_root=tmp_path / "experiments",
        experiment="earp-diagnostic-performance",
        ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
    )

    artifact = json.loads((outcome.run_dir / PERFORMANCE_RESULT_NAME).read_text())
    assert artifact["workload"]["parent_run_id"] == "quality-run"
    assert len(artifact["samples"]) == 2
    assert not any(path.name == "earp.result.v1.json" for path in outcome.run_dir.iterdir())
