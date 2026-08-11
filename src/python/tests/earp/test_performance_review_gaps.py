"""Regression contracts raised by the post-implementation evidence review.

These tests intentionally describe the required public artifact boundary.  They
are RED until the performance implementation closes the corresponding review
findings; no fixture manufactures a successful performance result.
"""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from types import SimpleNamespace

import pytest


SHA = "a" * 64
CANDIDATE = "b" * 40
TS = datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc)


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _quality_graph(root: Path, *, poisoned_record_config: bool = False) -> tuple[str, dict[str, object]]:
    """Create a writer-shaped, advertised quality graph with a rich manifest."""
    run_id = "quality-review"
    run = root / "runs" / run_id
    run.mkdir(parents=True)
    result = {
        "scenario": {"config_sha256": SHA, "query_call": "Engine.search", "effective_knobs": {"limit": 10, "alpha": 0.7}},
    }
    (run / "earp.result.v1.json").write_text(json.dumps(result) + "\n", encoding="utf-8")
    config = {"schema_version": "earp.v1", "campaign": "diagnostic", "scenario": {"query": {"call": "Engine.search"}}}
    (run / "config.resolved.yaml").write_text(json.dumps(config) + "\n", encoding="utf-8")
    resolved_workload = {
        "config_sha256": SHA,
        "query_call": "Engine.search",
        "effective_knobs": {"limit": 10, "alpha": 0.7},
        "corpus": {"snapshot": "corpus.json", "sha256": "c" * 64},
        "gold": {"path": "gold.json", "sha256": "d" * 64},
        "projections": {"fts": True, "vector": True, "graph": False},
        "embedder": {"provider": "deterministic", "model": "test"},
        "device": {"kind": "cpu"},
    }
    plan = {"repetitions": 20, "treatments": ["fresh_store"], "aggregation_rule": "descriptive_empirical_order_statistics", "invalid_result_policy": "typed_cell"}
    manifest = {
        "schema_version": "earp.workload-manifest.v1",
        "quality_parent": {"run_id": run_id, "evidence_family_id": run_id, "result_path": "earp.result.v1.json", "result_sha256": _digest(run / "earp.result.v1.json"), "candidate_sha": CANDIDATE, "clean": True},
        "resolved_config": {"path": "config.resolved.yaml", "sha256": _digest(run / "config.resolved.yaml"), "canonical_json": json.dumps(config, sort_keys=True)},
        "workload": resolved_workload,
        "performance_plan": plan,
    }
    manifest_path = run / "earp.workload-manifest.v1.json"
    manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8")
    poison = {"schema_version": "earp.v1", "campaign": "characterization", "scenario": {"query": {"call": "Engine.search"}}}
    record = {
        "code": {"git_sha": CANDIDATE},
        "config": {"resolved": poison if poisoned_record_config else config},
        "artifacts": [{"path": f"runs/{run_id}/earp.workload-manifest.v1.json", "sha256": _digest(manifest_path)}],
    }
    (run / "record.json").write_text(json.dumps(record, sort_keys=True) + "\n", encoding="utf-8")
    return run_id, manifest


def test_manifest_round_trips_every_execution_input_and_predeclared_plan(tmp_path: Path) -> None:
    from eval.performance.earp_adapter import load_earp_workload

    run_id, manifest = _quality_graph(tmp_path / "experiments")
    workload = load_earp_workload(tmp_path / "experiments", run_id)

    assert workload.resolved_workload == manifest["workload"]
    assert workload.predeclared_plan == manifest["performance_plan"]
    assert workload.as_document()["resolved_workload"] == manifest["workload"]


def test_cli_uses_manifest_config_and_refuses_plan_override(tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]) -> None:
    from eval.performance import cli

    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root, poisoned_record_config=True)
    called: list[object] = []
    monkeypatch.setattr(cli, "run_and_write_diagnostic_performance", lambda **kwargs: called.append(kwargs) or SimpleNamespace(run_id="performance", run_dir=root))

    # The poisoned shared record is not an authority.  The manifest config is
    # diagnostic, but the caller's one-repetition plan still conflicts with 20.
    assert cli.main(["diagnostic", "--experiments-root", str(root), "--quality-run", run_id, "--repetitions", "1", "--treatments", "fresh_store"]) == 2
    assert called == []
    assert "predeclared plan" in capsys.readouterr().err


def test_characterization_refuses_any_manifest_input_mismatch(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    from eval.performance import earp_adapter

    run_id, _ = _quality_graph(tmp_path / "experiments")
    workload = earp_adapter.load_earp_workload(tmp_path / "experiments", run_id)
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **_: pytest.fail("must refuse before execution"))
    config = {"corpus": {"data_root": "other", "snapshot": "other.json"}, "gold": {"path": "other-gold.json", "sha256": "e" * 64, "corpus_hash": "wrong", "qrels_version": "v2"}, "projections": {"fts": False}, "embedder": {"model": "other"}, "device": {"kind": "cuda"}, "scenario": {"query": {"call": "Engine.search"}}}
    with pytest.raises(ValueError, match="verified workload"):
        earp_adapter.run_characterization_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(1, ("fresh_store",)), scenario=SimpleNamespace(config_sha256=SHA, query_call="Engine.search"), config_doc=config)


def test_load_and_write_reject_tampered_or_forged_workload_reference(tmp_path: Path) -> None:
    from eval.performance.earp_adapter import PerformancePlan, RunSample, WorkloadRef, load_earp_workload, write_performance_result

    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root)
    workload = load_earp_workload(root, run_id)
    (root / "runs" / run_id / "earp.workload-manifest.v1.json").write_text("{}\n", encoding="utf-8")
    provenance = {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}
    sample = RunSample("fresh_store", 0, {"query": 1.0}, {"queries": 1}, {"fresh_database": True, "open_write_scope": "fresh_store"})
    with pytest.raises(ValueError, match="manifest"):
        write_performance_result(experiments_root=root, experiment="review", ts=TS, workload=workload, plan=PerformancePlan(1, ("fresh_store",)), samples=(sample,), execution_provenance=provenance)
    forged = WorkloadRef(parent_run_id=run_id, evidence_family_id=run_id, config_sha256=SHA, candidate_sha=CANDIDATE, query_call="Engine.search", effective_knobs={}, parent_manifest_path=f"runs/{run_id}/missing.json", parent_manifest_sha256=SHA)
    with pytest.raises(ValueError, match="manifest"):
        write_performance_result(experiments_root=root, experiment="forged", ts=TS, workload=forged, plan=PerformancePlan(1, ("fresh_store",)), samples=(sample,), execution_provenance=provenance)


def test_executor_exception_is_persisted_with_real_or_typed_unavailable_provenance(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    from eval.performance import earp_adapter

    run_id, _ = _quality_graph(tmp_path / "experiments")
    workload = earp_adapter.load_earp_workload(tmp_path / "experiments", run_id)
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: (_ for _ in ()).throw(TimeoutError("boom")))
    cells = earp_adapter.run_diagnostic_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(1, ("fresh_store",)), scenario=SimpleNamespace(config_sha256=SHA, query_call="Engine.search"), config_doc={}, experiments_root=tmp_path / "experiments", experiment="review", ts=TS)
    assert cells[0].status == "invalid"
    assert cells[0].invalidity["code"] == "timeout"
    assert cells[0].execution_provenance["candidate_sha"] != "unknown"
    assert cells[0].execution_provenance["candidate_sha"] == CANDIDATE or "candidate_sha" in cells[0].execution_provenance.get("unavailable", {})


def test_one_invalid_of_twenty_one_planned_cells_suppresses_treatment_summary(tmp_path: Path) -> None:
    from eval.performance.earp_adapter import PerformanceCell, PerformancePlan, RunSample, load_earp_workload, write_performance_result

    provenance = {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}
    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root)
    workload = load_earp_workload(root, run_id)
    def sample(rep: int) -> RunSample:
        return RunSample(
            "fresh_store",
            rep,
            {"query": 1.0},
            {"queries": 1},
            {"fresh_database": True, "open_write_scope": "fresh_store"},
        )
    cells = [PerformanceCell.complete(treatment="fresh_store", repetition=rep, samples=(sample(rep),), execution_provenance=provenance) for rep in range(20)]
    cells.append(PerformanceCell.invalid(treatment="fresh_store", repetition=20, raw_samples=(), invalidity={"code": "timeout", "message": "boom"}, execution_provenance=provenance))
    outcome = write_performance_result(experiments_root=root, experiment="review", ts=TS, workload=workload, plan=PerformancePlan(21, ("fresh_store",)), cells=cells)
    document = json.loads((outcome.run_dir / "performance.earp.v1.json").read_text(encoding="utf-8"))
    assert "fresh_store" not in document["summary"]


def test_observed_cost_v2_is_total_and_keeps_arm_local_evidence() -> None:
    from eval.earp.observed_cost import Observation, combine_arm_observations

    with pytest.raises(ValueError, match="provenance"):
        Observation("quality", SHA, {}, {}, {}, query_samples=({"query_id": "q", "outcome": "complete", "wall_ms": 1.0, "result_count": 0},))
    combined = combine_arm_observations(config_sha256=SHA, arms={"control": {"phases_ms": {}, "counts": {}, "storage": {}, "query_samples": [{"query_id": "q", "outcome": "complete", "wall_ms": 1.0, "result_count": 0}], "provenance": {"candidate_sha": CANDIDATE}}})
    assert combined["arms"]["control"]["query_samples"][0]["query_id"] == "q"
    assert combined["arms"]["control"]["provenance"]["candidate_sha"] == CANDIDATE


def test_blocked_characterization_still_writes_v2_unavailability_sidecar(tmp_path: Path) -> None:
    from eval.earp.characterize import run_characterization
    from test_characterization import _bed

    outcome = run_characterization(**_bed(tmp_path, data_root=tmp_path / "definitely-missing"))
    assert outcome.run_dir is not None
    observed = json.loads((outcome.run_dir / "earp.observed-cost.v2.json").read_text(encoding="utf-8"))
    assert observed["schema_version"] == "earp.observed-cost.v2"
    assert observed["unavailable"]["query_samples"]["code"]
    assert observed["provenance"]


def test_quality_writer_checks_the_entire_artifact_graph_on_idempotence(tmp_path: Path) -> None:
    from eval.earp.schema.models import RunVerdict
    from eval.earp.writer import write_run

    kwargs = {"experiment": "earp-diagnostic", "ts": TS, "config_doc": {"schema_version": "earp.v1", "campaign": "diagnostic"}, "experiments_root": tmp_path / "experiments", "verdict": RunVerdict.COMPLETE, "read": "review", "metrics": {}, "env": {"python": "3", "lockfile_sha256": None, "gpu": None, "key_deps": {}}, "corpus": {"source": None, "manifest_sha256": None, "datasets": []}, "seeds": {}, "cost_usd": 0.0}
    first = write_run(**kwargs, code={"git_sha": CANDIDATE, "dirty": False, "branch": "a", "baseline_commit": None})
    second = write_run(**kwargs, code={"git_sha": CANDIDATE, "dirty": False, "branch": "different-manifest-input", "baseline_commit": None})
    assert first.run_id == second.run_id
    assert second.blocker is not None
    assert second.blocker.code.value == "run_id_collision"


def test_manifest_and_performance_reject_nested_malformed_paths_and_schema(tmp_path: Path) -> None:
    from eval.performance.earp_adapter import load_earp_workload

    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root)
    manifest_path = root / "runs" / run_id / "earp.workload-manifest.v1.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["quality_parent"]["result_path"] = "../outside.json"
    manifest["workload"]["effective_knobs"] = ["not-a-mapping"]
    manifest["quality_parent"]["unexpected"] = True
    manifest_path.write_text(json.dumps(manifest) + "\n", encoding="utf-8")
    with pytest.raises(ValueError, match="manifest schema"):
        load_earp_workload(root, run_id)


def test_performance_schema_rejects_nested_bad_input_path_before_materialization() -> None:
    from eval.performance.earp_adapter import PerformanceSchemaError, _validate_performance_document

    document = {
        "schema_version": "performance.earp.v1", "run_id": "review", "scope": "repeated_performance_characterization",
        "relation": "same_candidate_reexecution", "quality_workload_sha256": SHA, "execution_workload_sha256": SHA,
        "workload": {}, "plan": {}, "parent_manifest": {"path": "../manifest.json", "sha256": SHA},
        "inputs": {"quality_result": {"path": ["not-a-path"], "sha256": "short"}}, "cells": [], "samples": [], "summary": {},
    }
    with pytest.raises(PerformanceSchemaError):
        _validate_performance_document(document)
