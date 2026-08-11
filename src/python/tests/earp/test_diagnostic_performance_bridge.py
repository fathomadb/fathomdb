"""The independent runner can execute a real diagnostic workload repeatedly."""

from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.performance.earp_adapter import PerformancePlan, run_diagnostic_repetitions
from performance_quality_fixture import diagnostic_quality_workload

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_bridge_runs_real_fresh_store_and_warm_diagnostic_cells(tmp_path: Path) -> None:
    scenario, config, root, workload = diagnostic_quality_workload(tmp_path)

    samples = run_diagnostic_repetitions(
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

    assert [(sample.treatment, sample.repetition) for sample in samples] == [
        ("fresh_store", 0),
        ("fresh_store_warm_query", 0),
    ]
    assert all({"open", "write", "query"} <= set(sample.phases_ms) for sample in samples)
    assert all(sample.treatment_witness["fresh_database"] is True for sample in samples)
    assert samples[0].treatment_witness["unmeasured_query_warmup"] is False
    assert samples[1].treatment_witness["unmeasured_query_warmup"] is True
    assert samples[1].treatment_witness["open_write_scope"] == "fresh_store"


def test_bridge_refuses_to_mislabel_same_process_work_as_process_cold(
    tmp_path: Path,
) -> None:
    scenario, config, root, workload = diagnostic_quality_workload(tmp_path)

    with pytest.raises(ValueError, match="fresh_store"):
        run_diagnostic_repetitions(
            workload=workload,
            plan=PerformancePlan(repetitions=1, treatments=("process_cold",)),
            scenario=scenario,
            config_doc=config,
            experiments_root=root,
            experiment="earp-diagnostic-performance",
            ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
        )


def test_bridge_preserves_raw_observation_when_one_cell_fails(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """FIX-3: a bridge exception becomes an invalid matrix cell, not data loss."""
    from eval.earp.runner import DiagnosticResult  # noqa: PLC0415
    from eval.earp.schema.models import RunVerdict  # noqa: PLC0415
    import eval.earp.runner as runner  # noqa: PLC0415

    scenario, config, root, workload = diagnostic_quality_workload(tmp_path)

    def fail_after_open(**_kwargs: object) -> DiagnosticResult:
        return DiagnosticResult(
            verdict=RunVerdict.FAILED,
            failure="TimeoutError: query exceeded 10 s",
            observed_cost={
                "phases_ms": {"open": 1.0, "write": 2.0},
                "counts": {"accepted": 1},
                "storage": {"database_bytes": 4},
            },
        )

    monkeypatch.setattr(runner, "run_diagnostic", fail_after_open)
    cells = run_diagnostic_repetitions(
        workload=workload,
        plan=PerformancePlan(repetitions=1, treatments=("fresh_store",)),
        scenario=scenario,
        config_doc=config,
        experiments_root=root,
        experiment="earp-diagnostic-performance",
        ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
    )
    assert cells[0].status == "invalid"
    assert cells[0].invalidity["code"] == "timeout"
    assert cells[0].raw_samples[0].phases_ms["open"] == 1.0
