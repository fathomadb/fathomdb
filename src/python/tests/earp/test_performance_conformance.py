"""Red contracts for the eight 0.8.24 performance-evidence review fixes.

These tests describe the durable artifact boundary. Success paths use a real
quality run and the adapter's verified-manifest loader; they never construct a
publication workload by hand.
"""

from __future__ import annotations

import json
import hashlib
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.earp.observed_cost import Observation
from eval.performance.earp_adapter import (
    PERFORMANCE_RESULT_NAME,
    PerformancePlan,
    RunSample,
    write_performance_result,
)
from performance_quality_fixture import diagnostic_quality_workload

TS = datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc)
SHA = "a" * 64


pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def _workload(tmp_path: Path):
    """Return only a writer-originated quality workload verified by the adapter."""
    return diagnostic_quality_workload(tmp_path)[3]


def _execution_provenance(*, candidate_sha: str = "b" * 40) -> dict[str, object]:
    return {
        "candidate_sha": candidate_sha,
        "clean": True,
        "command": "fathomdb-performance diagnostic --quality-run quality-run",
        "lockfile_sha256": "c" * 64,
        "toolchain": {"python": "3.12.0", "fathomdb": "0.8.24"},
        "device": {"kind": "cpu"},
        "fixtures": {"fixture.jsonl": "d" * 64},
    }


def _declared_samples(*, fresh_query_ms: float = 1.0) -> tuple[RunSample, RunSample]:
    """One sample for every treatment declared by the quality manifest."""
    return (
        RunSample("fresh_store", 0, {"query": fresh_query_ms}, {"results": 1}),
        RunSample("fresh_store_warm_query", 0, {"query": 0.5}, {"results": 1}),
    )


def test_complete_cell_carries_distinct_execution_provenance(tmp_path: Path) -> None:
    """FIX-2: quality identity may never be copied into an execution cell."""
    result = write_performance_result(
        experiments_root=tmp_path / "experiments",
        experiment="earp-performance",
        ts=TS,
        workload=_workload(tmp_path),
        plan=PerformancePlan(repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")),
        samples=_declared_samples(),
        execution_provenance=_execution_provenance(candidate_sha="e" * 40),
    )

    document = json.loads((result.run_dir / PERFORMANCE_RESULT_NAME).read_text())
    cell = document["cells"][0]
    assert cell["execution_provenance"]["candidate_sha"] == "e" * 40
    assert document["relation"] == "cross_candidate_reexecution"
    assert document["execution_workload_sha256"] != document["quality_workload_sha256"]
    assert cell["status"] == "invalid"
    assert cell["invalidity"]["code"] == "cross_candidate"


def test_runner_persists_a_typed_invalid_cell_without_losing_prior_raw_samples(
    tmp_path: Path,
) -> None:
    """FIX-3: the planned matrix survives a timeout/error as typed evidence."""
    from eval.performance.earp_adapter import PerformanceCell  # noqa: PLC0415

    workload = _workload(tmp_path)
    provenance = _execution_provenance(candidate_sha=workload.candidate_sha)
    complete = PerformanceCell.complete(
        treatment="fresh_store",
        repetition=0,
        samples=(RunSample("fresh_store", 0, {"query": 1.0}, {"results": 1}),),
        execution_provenance=provenance,
    )
    timed_out = PerformanceCell.invalid(
        treatment="fresh_store_warm_query",
        repetition=0,
        raw_samples=(RunSample("fresh_store_warm_query", 0, {"open": 0.5}, {}),),
        invalidity={"code": "timeout", "message": "query exceeded 10 s"},
        execution_provenance=provenance,
    )
    result = write_performance_result(
        experiments_root=tmp_path / "experiments",
        experiment="earp-performance",
        ts=TS,
        workload=workload,
        plan=PerformancePlan(
            repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")
        ),
        cells=(complete, timed_out),
    )

    document = json.loads((result.run_dir / PERFORMANCE_RESULT_NAME).read_text())
    assert [(cell["treatment"], cell["status"]) for cell in document["cells"]] == [
        ("fresh_store", "complete"),
        ("fresh_store_warm_query", "invalid"),
    ]
    assert document["cells"][1]["raw_samples"] == [
        {"treatment": "fresh_store_warm_query", "repetition": 0, "phases_ms": {"open": 0.5}, "counts": {}}
    ]
    assert "fresh_store_warm_query" not in document["summary"]


def test_observed_cost_marks_unavailable_fields_and_keeps_per_query_samples() -> None:
    """FIX-6: absent detail is explicit, never silently omitted."""
    document = Observation(
        evidence_family_id="quality-run",
        config_sha256=SHA,
        phases_ms={"open": 1.0, "write": 2.0},
        counts={"accepted": 1},
        storage={"database_bytes": 4, "wal_bytes": 0, "shm_bytes": 0},
        query_samples=(
            {
                "query_id": "q-1",
                "wall_ms": 0.75,
                "result_count": 1,
                "outcome": "complete",
            },
        ),
        unavailable={
            "embedding_ready_ms": {
                "code": "not_configured",
                "message": "dense projection was not requested",
            },
            "engine_trace": {"code": "not_exposed", "message": "binding has no trace hook"},
        },
        provenance={
            "candidate_sha": "b" * 40,
            "clean": True,
            "toolchain": {"python": "3.12.0"},
            "device": {"kind": "cpu"},
        },
    ).as_document()

    assert document["query_samples"][0]["query_id"] == "q-1"
    assert document["unavailable"]["engine_trace"]["code"] == "not_exposed"
    assert document["provenance"]["candidate_sha"] == "b" * 40


def test_unverified_or_incomplete_evidence_cannot_claim_repeated_performance(
    tmp_path: Path,
) -> None:
    """FIX-7: an artifact cannot obtain a benchmark-looking scope by omission."""
    with pytest.raises(ValueError, match="manifest|execution provenance|complete matrix"):
        write_performance_result(
            experiments_root=tmp_path / "experiments",
            experiment="earp-performance",
            ts=TS,
            workload=_workload(tmp_path),
            plan=PerformancePlan(repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")),
            samples=_declared_samples(),
        )


def test_writer_requires_complete_execution_provenance(tmp_path: Path) -> None:
    """FIX-2: a candidate SHA alone is not an execution provenance record."""
    with pytest.raises(ValueError, match="execution provenance"):
        write_performance_result(
            experiments_root=tmp_path / "experiments",
            experiment="earp-performance",
            ts=TS,
            workload=_workload(tmp_path),
            plan=PerformancePlan(repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")),
            samples=_declared_samples(),
            execution_provenance={"candidate_sha": "b" * 40},
        )


def test_invalid_performance_schema_leaves_no_run_or_index(tmp_path: Path) -> None:
    """FIX-8: schema rejection leaves the quality parent byte-for-byte intact."""
    from eval.performance.earp_adapter import PerformanceSchemaError  # noqa: PLC0415

    workload = _workload(tmp_path)
    root = tmp_path / "experiments"
    parent_dir = root / "runs" / workload.parent_run_id
    parent_bytes = {
        path.relative_to(parent_dir): path.read_bytes()
        for path in parent_dir.iterdir()
        if path.is_file()
    }
    index_before = (root / "index.jsonl").read_bytes()
    with pytest.raises(PerformanceSchemaError):
        write_performance_result(
            experiments_root=root,
            experiment="earp-performance",
            ts=TS,
            workload=workload,
            plan=PerformancePlan(repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")),
            samples=_declared_samples(),
            execution_provenance={**_execution_provenance(), "device": {"kind": "unknown"}},
        )
    assert (root / "index.jsonl").read_bytes() == index_before
    assert {
        path.relative_to(parent_dir): path.read_bytes()
        for path in parent_dir.iterdir()
        if path.is_file()
    } == parent_bytes
    assert not list((root / "runs").glob("earp-performance*/**/*"))


def test_index_failure_cleans_staged_performance_output(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """FIX-8: an index failure may not leave a complete-looking artifact."""
    from eval.earp._experiments import lib as _lib  # noqa: PLC0415

    def fail_append(*_args: object, **_kwargs: object) -> None:
        raise OSError("simulated index failure")

    workload = _workload(tmp_path)
    monkeypatch.setattr(_lib, "append_index", fail_append)
    root = tmp_path / "experiments"
    with pytest.raises(OSError, match="index failure"):
        write_performance_result(
            experiments_root=root,
            experiment="earp-performance",
            ts=TS,
            workload=workload,
            plan=PerformancePlan(
                repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")
            ),
            samples=_declared_samples(),
            execution_provenance=_execution_provenance(),
        )
    performance_records = list((root / "runs").glob("earp-performance*/record.json"))
    assert not performance_records
    assert not list((root / "runs").glob("earp-performance*/performance.earp.v1.json"))


def test_performance_artifact_is_validated_digest_linked_and_idempotent(
    tmp_path: Path,
) -> None:
    """FIX-8: equal evidence is idempotent; differing evidence is a typed collision."""
    from eval.performance.earp_adapter import PerformanceCollision  # noqa: PLC0415

    kwargs = {
        "experiments_root": tmp_path / "experiments",
        "experiment": "earp-performance",
        "ts": TS,
        "workload": _workload(tmp_path),
        "plan": PerformancePlan(repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")),
        "samples": _declared_samples(),
        "execution_provenance": _execution_provenance(),
    }
    first = write_performance_result(**kwargs)
    second = write_performance_result(**kwargs)
    assert second == first

    record = json.loads((first.run_dir / "record.json").read_text())
    sidecar_bytes = (first.run_dir / PERFORMANCE_RESULT_NAME).read_bytes()
    advertised = next(
        artifact
        for artifact in record["artifacts"]
        if artifact["path"] == f"runs/{first.run_id}/{PERFORMANCE_RESULT_NAME}"
    )
    assert advertised["sha256"] == hashlib.sha256(sidecar_bytes).hexdigest()
    document = json.loads(sidecar_bytes)
    assert PERFORMANCE_RESULT_NAME not in document.get("artifact_digests", {})
    assert document["parent_manifest"]["sha256"]
    assert document["inputs"]["quality_result"]["sha256"]

    with pytest.raises(PerformanceCollision):
        write_performance_result(
            **{**kwargs, "samples": _declared_samples(fresh_query_ms=2.0)}
        )
