"""A diagnostic performance execution publishes only the independent artifact."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.performance.earp_adapter import (
    PERFORMANCE_RESULT_NAME,
    PerformancePlan,
    run_and_write_diagnostic_performance,
)
from performance_quality_fixture import diagnostic_quality_workload

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_diagnostic_performance_writes_linked_independent_artifact(tmp_path: Path) -> None:
    scenario, config, root, workload = diagnostic_quality_workload(tmp_path)
    outcome = run_and_write_diagnostic_performance(
        workload=workload,
        plan=PerformancePlan(
            repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")
        ),
        scenario=scenario,
        config_doc=config,
        experiments_root=root,
        experiment="earp-diagnostic-performance",
        ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
    )

    artifact = json.loads((outcome.run_dir / PERFORMANCE_RESULT_NAME).read_text())
    assert artifact["workload"]["parent_run_id"] == "quality-run"
    assert len(artifact["samples"]) == 2
    assert not any(path.name == "earp.result.v1.json" for path in outcome.run_dir.iterdir())
