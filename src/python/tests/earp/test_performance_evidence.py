"""0.8.24 performance-evidence contracts.

These tests deliberately exercise the two evidence planes without opening a
native engine: correctness of the durable artifact contract must not depend on
whether a developer has a built extension or a cached embedder.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.earp.observed_cost import (
    OBSERVED_COST_NAME,
    Observation,
    capture_sqlite_storage,
)
from eval.performance.earp_adapter import (
    PERFORMANCE_RESULT_NAME,
    PerformancePlan,
    RunSample,
    WorkloadRef,
    summarize_samples,
    write_performance_result,
)

TS = datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc)
SHA = "a" * 64


def test_observed_cost_sidecar_is_explicitly_one_run_evidence(tmp_path: Path) -> None:
    db = tmp_path / "scenario.db"
    db.write_bytes(b"main")
    (tmp_path / "scenario.db-wal").write_bytes(b"wal")

    observation = Observation(
        evidence_family_id="quality-run-1",
        config_sha256=SHA,
        phases_ms={"open": 1.25, "write": 3.5, "query": 0.75},
        counts={"accepted": 2, "queries": 1, "results": 1},
        storage=capture_sqlite_storage(db),
    )

    document = observation.as_document()
    assert document["schema_version"] == "earp.observed-cost.v1"
    assert document["scope"] == "one_run_observation"
    assert document["evidence_family_id"] == "quality-run-1"
    assert document["storage"] == {"database_bytes": 4, "wal_bytes": 3, "shm_bytes": 0}
    assert OBSERVED_COST_NAME.endswith(".json")


def test_repeated_performance_artifact_links_to_exact_quality_workload(tmp_path: Path) -> None:
    workload = WorkloadRef(
        parent_run_id="quality-run-1",
        evidence_family_id="quality-run-1",
        config_sha256=SHA,
        candidate_sha="b" * 40,
        query_call="Engine.search",
        effective_knobs={"limit": 10, "rerank_depth": 20},
    )
    plan = PerformancePlan(repetitions=2, treatments=("fresh_store", "warm"))
    samples = (
        RunSample("fresh_store", 0, {"query": 4.0}, {"results": 1}),
        RunSample("fresh_store", 1, {"query": 6.0}, {"results": 1}),
        RunSample("warm", 0, {"query": 2.0}, {"results": 1}),
        RunSample("warm", 1, {"query": 3.0}, {"results": 1}),
    )
    summary = summarize_samples(samples)
    assert summary["fresh_store"]["query"]["p95_ms"] == 6.0
    assert summary["warm"]["query"]["p50_ms"] == 2.5

    result = write_performance_result(
        experiments_root=tmp_path / "experiments",
        experiment="earp-performance",
        ts=TS,
        workload=workload,
        plan=plan,
        samples=samples,
    )

    artifact = json.loads((result.run_dir / PERFORMANCE_RESULT_NAME).read_text(encoding="utf-8"))
    assert artifact["workload"] == workload.as_document()
    assert artifact["plan"] == plan.as_document()
    assert artifact["summary"] == summary
    record = json.loads((result.run_dir / "record.json").read_text(encoding="utf-8"))
    assert f"runs/{result.run_id}/{PERFORMANCE_RESULT_NAME}" in record["artifacts"]


def test_performance_result_stages_sidecar_before_index_append(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    from eval.earp._experiments import lib as _lib

    observed: dict[str, bool] = {}
    original = _lib.append_index

    def spy(record: dict[str, object], *, index_path: str | Path | None = None) -> None:
        run_id = str(record["run_id"])
        observed["sidecar"] = (tmp_path / "experiments" / "runs" / run_id / PERFORMANCE_RESULT_NAME).is_file()
        original(record, index_path=index_path)

    monkeypatch.setattr(_lib, "append_index", spy)
    write_performance_result(
        experiments_root=tmp_path / "experiments",
        experiment="earp-performance",
        ts=TS,
        workload=WorkloadRef(
            parent_run_id="quality-run-1",
            evidence_family_id="quality-run-1",
            config_sha256=SHA,
            candidate_sha="b" * 40,
            query_call="Engine.search_text_only",
            effective_knobs={"limit": 10},
        ),
        plan=PerformancePlan(repetitions=1, treatments=("warm",)),
        samples=(RunSample("warm", 0, {"query": 1.0}, {"results": 1}),),
    )
    assert observed == {"sidecar": True}
