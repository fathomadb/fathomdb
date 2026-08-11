"""Independent performance runner execution discipline."""

from __future__ import annotations

import pytest

from eval.performance.earp_adapter import (
    PerformancePlan,
    RunSample,
    WorkloadRef,
    run_repetitions,
)


def _workload() -> WorkloadRef:
    return WorkloadRef(
        parent_run_id="quality-run-1",
        evidence_family_id="quality-run-1",
        config_sha256="a" * 64,
        candidate_sha="b" * 40,
        query_call="Engine.search_text_only",
        effective_knobs={"limit": 10},
    )


def test_runner_executes_every_declared_cell_once() -> None:
    calls: list[tuple[str, int, str]] = []

    def execute(workload: WorkloadRef, treatment: str, repetition: int) -> RunSample:
        calls.append((treatment, repetition, workload.config_sha256))
        return RunSample(treatment, repetition, {"query": repetition + 1.0}, {"results": 1})

    samples = run_repetitions(
        workload=_workload(),
        plan=PerformancePlan(
            repetitions=2, treatments=("fresh_store", "fresh_store_warm_query")
        ),
        execute=execute,
    )

    assert calls == [
        ("fresh_store", 0, "a" * 64),
        ("fresh_store", 1, "a" * 64),
        ("fresh_store_warm_query", 0, "a" * 64),
        ("fresh_store_warm_query", 1, "a" * 64),
    ]
    assert [(sample.treatment, sample.repetition) for sample in samples] == [
        ("fresh_store", 0),
        ("fresh_store", 1),
        ("fresh_store_warm_query", 0),
        ("fresh_store_warm_query", 1),
    ]


def test_runner_rejects_an_executor_that_changes_the_declared_cell() -> None:
    def execute(_workload: WorkloadRef, _treatment: str, _repetition: int) -> RunSample:
        return RunSample("fresh_store_warm_query", 0, {"query": 1.0}, {})

    with pytest.raises(ValueError, match="declared cell"):
        run_repetitions(
            workload=_workload(),
            plan=PerformancePlan(repetitions=1, treatments=("fresh_store",)),
            execute=execute,
        )
