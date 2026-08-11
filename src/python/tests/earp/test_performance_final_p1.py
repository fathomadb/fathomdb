"""Final P1 contracts for 0.8.24 performance evidence."""

from __future__ import annotations

import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from eval.earp.observed_cost import Observation
from eval.earp.schema.validate import validate
from eval.performance.earp_adapter import PerformancePlan, WorkloadRef


SHA = "a" * 64


def _workload(*, plan: dict[str, object] | None = None) -> WorkloadRef:
    return WorkloadRef(
        parent_run_id="quality-run",
        evidence_family_id="quality-run",
        config_sha256=SHA,
        candidate_sha="b" * 40,
        query_call="Engine.search",
        effective_knobs={"limit": 10},
        parent_manifest_path="runs/quality-run/earp.workload-manifest.v1.json",
        parent_manifest_sha256=SHA,
        quality_result_path="earp.result.v1.json",
        quality_result_sha256=SHA,
        resolved_config_path="config.resolved.yaml",
        resolved_config_sha256=SHA,
        quality_clean=True,
        resolved_workload={
            "config_sha256": SHA,
            "query_call": "Engine.search",
            "effective_knobs": {"limit": 10},
            "corpus": {"snapshot": "corpus.json", "data_root": "data"},
            "gold": {"path": "gold.json", "sha256": SHA},
            "projections": {},
            "embedder": {},
            "device": {"kind": "cpu"},
        },
        predeclared_plan=plan or {},
    )


def test_every_actual_plan_is_exact_even_for_one_repetition() -> None:
    from eval.performance.earp_adapter import _require_predeclared_plan

    workload = _workload(
        plan={
            "kind": "predeclared_repeated",
            "repetitions": 1,
            "treatments": ["fresh_store"],
        }
    )

    with pytest.raises(ValueError, match="predeclared manifest plan"):
        _require_predeclared_plan(
            workload, PerformancePlan(1, ("fresh_store_warm_query",))
        )


def test_workload_identity_includes_every_artifact_edge() -> None:
    workload = _workload()
    changed = WorkloadRef(
        **{
            **workload.__dict__,
            "quality_result_path": "subdir/other-result.json",
            "quality_result_sha256": "c" * 64,
            "resolved_config_path": "subdir/other-config.yaml",
            "resolved_config_sha256": "d" * 64,
        }
    )

    assert changed.as_document() != workload.as_document()


def test_characterization_readmits_before_opening_an_arm(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    from eval.performance import earp_adapter

    called: list[object] = []
    monkeypatch.setattr(
        earp_adapter,
        "_verify_workload_reference",
        lambda *_: (_ for _ in ()).throw(ValueError("manifest changed")),
    )
    monkeypatch.setattr(
        "eval.earp.characterize.execute_arm", lambda **_: called.append(True)
    )

    with pytest.raises(ValueError, match="manifest changed"):
        earp_adapter.run_characterization_repetitions(
            workload=_workload(),
            plan=PerformancePlan(1, ("fresh_store",)),
            scenario=SimpleNamespace(
                config_sha256=SHA,
                query_call="Engine.search",
                query_params={"limit": 10},
            ),
            config_doc={
                "corpus": {"snapshot": "corpus.json", "data_root": "data"},
                "gold": {"path": "gold.json", "sha256": SHA},
                "projections": {},
                "embedder": {},
                "device": {"kind": "cpu"},
            },
            experiments_root=tmp_path,
        )
    assert called == []


def test_observed_cost_schema_accepts_measured_or_typed_unavailable() -> None:
    schema_path = Path(__file__).resolve().parents[2] / "eval/earp/schema/earp.observed-cost.v2.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    document = Observation(
        evidence_family_id="quality-run",
        config_sha256=SHA,
        phases_ms={"open": 1.0, "ingest": 2.0, "query": 3.0},
        counts={"documents": 1, "queries": 1},
        storage={"database_bytes": 1, "wal_bytes": 0, "shm_bytes": 0},
        query_samples=(
            {
                "query_id": "q",
                "outcome": "complete",
                "wall_ms": 3.0,
                "result_count": 1,
            },
        ),
        unavailable={"engine_trace": {"code": "not_exposed", "message": "none"}},
        provenance={"candidate_sha": "b" * 40, "clean": True, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}},
    ).as_document()

    assert validate(document, schema) == []


def test_observed_cost_rejects_provenance_sentinels() -> None:
    with pytest.raises(ValueError, match="sentinel"):
        Observation(
            evidence_family_id="quality-run",
            config_sha256=SHA,
            phases_ms={},
            counts={},
            storage={},
            unavailable={},
            provenance={"candidate_sha": "unknown"},
        )


def test_performance_schema_rejects_unknown_nested_raw_sample_field() -> None:
    schema_path = Path(__file__).resolve().parents[2] / "eval/performance/schema/performance.earp.v1.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    document = {
        "schema_version": "performance.earp.v1",
        "run_id": "performance-run",
        "scope": "repeated_performance_characterization",
        "relation": "same_candidate_reexecution",
        "quality_workload_sha256": SHA,
        "execution_workload_sha256": SHA,
        "workload": {"config_sha256": SHA, "query_call": "Engine.search", "effective_knobs": {}},
        "plan": {"repetitions": 1, "treatments": ["fresh_store"]},
        "parent_manifest": {"path": "runs/quality/earp.workload-manifest.v1.json", "sha256": SHA},
        "inputs": {
            "quality_result": {"path": "earp.result.v1.json", "sha256": SHA},
            "resolved_config": {"path": "config.resolved.yaml", "sha256": SHA},
        },
        "cells": [{
            "treatment": "fresh_store",
            "repetition": 0,
            "status": "complete",
            "raw_samples": [{
                "treatment": "fresh_store",
                "repetition": 0,
                "phases_ms": {"query": 1.0},
                "counts": {"queries": 1},
                "unexpected": True,
            }],
            "execution_provenance": {"candidate_sha": "b" * 40},
        }],
        "samples": [],
        "summary": {},
    }

    assert validate(document, schema)
