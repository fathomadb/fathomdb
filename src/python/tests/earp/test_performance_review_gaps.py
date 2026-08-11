"""Regression contracts raised by the post-implementation evidence review.

These tests intentionally describe the required public artifact boundary.  They
are RED until the performance implementation closes the corresponding review
findings; no fixture manufactures a successful performance result.
"""

from __future__ import annotations

import hashlib
import json
from copy import deepcopy
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
    plan = {
        "repetitions": 20,
        "treatments": ["fresh_store"],
        "warmup_rule": "fresh_store has no measured warmup",
        "aggregation_rule": "descriptive_empirical_order_statistics",
        "invalid_result_policy": "typed_cell",
        "command": "fathomdb-performance diagnostic",
        "kind": "descriptive_nonclaim",
    }
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


def _rewrite_manifest_advertisement(root: Path, run_id: str, manifest: dict[str, object]) -> None:
    """Mutate a manifest while preserving the record → manifest digest edge."""
    run = root / "runs" / run_id
    manifest_path = run / "earp.workload-manifest.v1.json"
    manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8")
    record_path = run / "record.json"
    record = json.loads(record_path.read_text(encoding="utf-8"))
    record["artifacts"][0]["sha256"] = _digest(manifest_path)
    record_path.write_text(json.dumps(record, sort_keys=True) + "\n", encoding="utf-8")


def _characterization_graph(root: Path) -> tuple[str, dict[str, object]]:
    """Create a complete characterization-shaped graph for bridge admission."""
    run_id, manifest = _quality_graph(root)
    run = root / "runs" / run_id
    workload = manifest["workload"]
    config = {
        "schema_version": "earp.v1",
        "campaign": "characterization",
        "corpus": workload["corpus"],
        "gold": workload["gold"],
        "projections": workload["projections"],
        "embedder": workload["embedder"],
        "device": workload["device"],
        "scenario": {"query": {"call": workload["query_call"], **workload["effective_knobs"]}},
    }
    (run / "config.resolved.yaml").write_text(json.dumps(config) + "\n", encoding="utf-8")
    result = {"scenario": {"config_sha256": SHA, "query_call": workload["query_call"], "effective_knobs": workload["effective_knobs"]}}
    (run / "earp.result.v1.json").write_text(json.dumps(result) + "\n", encoding="utf-8")
    manifest["resolved_config"] = {"path": "config.resolved.yaml", "sha256": _digest(run / "config.resolved.yaml"), "canonical_json": json.dumps(config, sort_keys=True)}
    manifest["quality_parent"]["result_sha256"] = _digest(run / "earp.result.v1.json")
    manifest["performance_plan"]["command"] = "fathomdb-performance characterization"
    _rewrite_manifest_advertisement(root, run_id, manifest)
    return run_id, manifest


def _matching_scenario(manifest: dict[str, object]) -> SimpleNamespace:
    """Complete normalized scenario used only by legacy success fixtures."""
    workload = manifest["workload"]
    return SimpleNamespace(
        config_sha256=SHA,
        query_call=workload["query_call"],
        query_params=dict(workload["effective_knobs"]),
        retrieval_mode="text",
        max_measurable_k=10,
        use_default_embedder=True,
    )


def test_manifest_round_trips_every_execution_input_and_predeclared_plan(tmp_path: Path) -> None:
    from eval.performance.earp_adapter import load_earp_workload

    run_id, manifest = _quality_graph(tmp_path / "experiments")
    workload = load_earp_workload(tmp_path / "experiments", run_id)

    serialized = workload.as_document()
    assert serialized["workload"] == manifest["workload"]
    assert serialized["performance_plan"] == manifest["performance_plan"]


def test_cli_uses_manifest_config_and_refuses_plan_override(tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]) -> None:
    from eval.performance import cli

    root = tmp_path / "experiments"
    run_id, manifest = _quality_graph(root, poisoned_record_config=True)
    called: list[dict[str, object]] = []
    monkeypatch.setattr(cli, "run_and_write_diagnostic_performance", lambda **kwargs: called.append(kwargs) or SimpleNamespace(run_id="performance", run_dir=root))

    # The poisoned shared record is not an authority: a matching invocation
    # uses the manifest's diagnostic config and its serialized workload.
    assert cli.main(["diagnostic", "--experiments-root", str(root), "--quality-run", run_id, "--repetitions", "20", "--treatments", "fresh_store"]) == 0
    assert called[0]["config_doc"] == json.loads(manifest["resolved_config"]["canonical_json"])
    assert called[0]["workload"].as_document()["workload"] == manifest["workload"]

    # A caller may not substitute a different plan after admission.
    called.clear()
    assert cli.main(["diagnostic", "--experiments-root", str(root), "--quality-run", run_id, "--repetitions", "1", "--treatments", "fresh_store"]) == 2
    assert called == []
    assert "predeclared plan" in capsys.readouterr().err


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

    root = tmp_path / "experiments"
    run_id, manifest = _quality_graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: (_ for _ in ()).throw(TimeoutError("boom")))
    cells = earp_adapter.run_diagnostic_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(20, ("fresh_store",)), scenario=_matching_scenario(manifest), config_doc=json.loads(manifest["resolved_config"]["canonical_json"]), experiments_root=root, experiment="review", ts=TS)
    assert cells[0].status == "invalid"
    assert cells[0].invalidity["code"] == "timeout"
    _assert_total_provenance(cells[0].execution_provenance)


def _assert_total_provenance(provenance: object) -> None:
    """Every provenance component is concrete or has a typed unavailable reason."""
    assert isinstance(provenance, dict)
    unavailable = provenance.get("unavailable", {})
    assert isinstance(unavailable, dict)
    for name in ("candidate_sha", "clean", "command", "lockfile_sha256", "toolchain", "device", "fixtures"):
        if name in provenance:
            value = provenance[name]
            assert value is not None
            if isinstance(value, str):
                assert value not in {"", "unknown", "unavailable", "0" * 64}
        else:
            reason = unavailable[name]
            assert isinstance(reason.get("code"), str) and reason["code"]
            assert isinstance(reason.get("message"), str) and reason["message"]


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
    cells = [PerformanceCell.complete(treatment="fresh_store", repetition=rep, samples=(sample(rep),), execution_provenance=provenance) for rep in range(19)]
    cells.append(PerformanceCell.invalid(treatment="fresh_store", repetition=19, raw_samples=(), invalidity={"code": "timeout", "message": "boom"}, execution_provenance=provenance))
    outcome = write_performance_result(experiments_root=root, experiment="review", ts=TS, workload=workload, plan=PerformancePlan(20, ("fresh_store",)), cells=cells)
    document = json.loads((outcome.run_dir / "performance.earp.v1.json").read_text(encoding="utf-8"))
    assert "fresh_store" not in document["summary"]


def test_observed_cost_v2_is_total_and_keeps_arm_local_evidence() -> None:
    from eval.earp.observed_cost import Observation, combine_arm_observations

    with pytest.raises(ValueError, match="provenance"):
        Observation("quality", SHA, {}, {}, {}, query_samples=({"query_id": "q", "outcome": "complete", "wall_ms": 1.0, "result_count": 0},))
    combined = combine_arm_observations(config_sha256=SHA, arms={"control": {"phases_ms": {}, "counts": {}, "storage": {}, "query_samples": [{"query_id": "q", "outcome": "complete", "wall_ms": 1.0, "result_count": 0}], "unavailable": {"engine_trace": {"code": "not_exposed", "message": "no trace hook"}}, "provenance": {"candidate_sha": CANDIDATE}}})
    assert combined["arms"]["control"]["query_samples"][0]["query_id"] == "q"
    assert combined["arms"]["control"]["provenance"]["candidate_sha"] == CANDIDATE
    assert combined["arms"]["control"]["unavailable"]["engine_trace"]["code"] == "not_exposed"


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


@pytest.mark.parametrize(
    "mutation",
    [
        lambda manifest: manifest["quality_parent"].__setitem__("result_path", "../outside.json"),
        lambda manifest: manifest["workload"].__setitem__("effective_knobs", ["not-a-mapping"]),
        lambda manifest: manifest["quality_parent"].__setitem__("unexpected", True),
    ],
    ids=("traversal-path", "nonmapping-knobs", "unknown-nested-field"),
)
def test_manifest_rejects_one_nested_malformed_value_after_valid_baseline(
    tmp_path: Path, mutation: object
) -> None:
    from eval.performance.earp_adapter import load_earp_workload

    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root)
    load_earp_workload(root, run_id)  # Valid baseline graph before one mutation.
    manifest_path = root / "runs" / run_id / "earp.workload-manifest.v1.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert callable(mutation)
    mutation(manifest)
    _rewrite_manifest_advertisement(root, run_id, manifest)
    with pytest.raises(ValueError, match="manifest schema"):
        load_earp_workload(root, run_id)


def test_performance_schema_rejects_nested_bad_input_path_before_materialization() -> None:
    from eval.earp.schema.validate import validate

    schema_path = Path(__file__).resolve().parents[2] / "eval/performance/schema/performance.earp.v1.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    baseline = {
        "schema_version": "performance.earp.v1", "run_id": "review", "scope": "repeated_performance_characterization",
        "relation": "same_candidate_reexecution", "quality_workload_sha256": SHA, "execution_workload_sha256": SHA,
        "workload": {"config_sha256": SHA, "query_call": "Engine.search", "effective_knobs": {}},
        "plan": {"repetitions": 1, "treatments": ["fresh_store"]},
        "parent_manifest": {"path": "runs/quality/earp.workload-manifest.v1.json", "sha256": SHA},
        "inputs": {"quality_result": {"path": "earp.result.v1.json", "sha256": SHA}, "resolved_config": {"path": "config.resolved.yaml", "sha256": SHA}},
        "cells": [{"treatment": "fresh_store", "repetition": 0, "status": "complete", "raw_samples": [{"treatment": "fresh_store", "repetition": 0, "phases_ms": {"query": 1.0}, "counts": {"queries": 1}, "treatment_witness": {"fresh_database": True, "open_write_scope": "fresh_store"}}], "execution_provenance": {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}}],
        "samples": [{"treatment": "fresh_store", "repetition": 0, "phases_ms": {"query": 1.0}, "counts": {"queries": 1}, "treatment_witness": {"fresh_database": True, "open_write_scope": "fresh_store"}}],
        "summary": {"fresh_store": {"query": {"n": 1, "min": 1.0, "max": 1.0, "median": 1.0}}},
    }
    assert validate(baseline, schema) == []
    document = deepcopy(baseline)
    document["inputs"]["quality_result"]["path"] = "../outside.json"
    assert validate(document, schema)


def test_diagnostic_matching_baseline_reaches_executor_then_plan_drift_refuses(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Plan refusal is independent of scenario/config admission."""
    from eval.performance import earp_adapter
    from eval.earp.schema.models import RunVerdict

    root = tmp_path / "experiments"
    run_id, manifest = _quality_graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    calls: list[object] = []
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **kwargs: calls.append(kwargs) or SimpleNamespace(verdict=RunVerdict.COMPLETE, observed_cost={"phases_ms": {"query": 1.0}, "counts": {"queries": 1}}, failure=None, blockers=()))
    scenario = SimpleNamespace(config_sha256=SHA, query_call="Engine.search", query_params={"limit": 10, "alpha": 0.7}, retrieval_mode="text", max_measurable_k=10, use_default_embedder=True)
    matching = earp_adapter.PerformancePlan(manifest["performance_plan"]["repetitions"], tuple(manifest["performance_plan"]["treatments"]))
    earp_adapter.run_diagnostic_repetitions(workload=workload, plan=matching, scenario=scenario, config_doc=json.loads(manifest["resolved_config"]["canonical_json"]), experiments_root=root, experiment="review", ts=TS)
    assert len(calls) == 20
    calls.clear()
    with pytest.raises(ValueError, match="plan"):
        earp_adapter.run_diagnostic_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(1, ("fresh_store",)), scenario=scenario, config_doc=json.loads(manifest["resolved_config"]["canonical_json"]), experiments_root=root, experiment="review", ts=TS)
    assert calls == []


def test_characterization_matching_baseline_reaches_executor_then_plan_drift_refuses(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    from eval.performance import earp_adapter

    root = tmp_path / "experiments"
    run_id, manifest = _characterization_graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    calls: list[object] = []
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **kwargs: calls.append(kwargs) or SimpleNamespace(blocker=None, observed_cost={"phases_ms": {"query": 1.0}, "counts": {"queries": 1}}))
    scenario = _matching_scenario(manifest)
    config = {key: deepcopy(manifest["workload"][key]) for key in ("corpus", "gold", "projections", "embedder", "device")}
    matching = earp_adapter.PerformancePlan(manifest["performance_plan"]["repetitions"], tuple(manifest["performance_plan"]["treatments"]))
    earp_adapter.run_characterization_repetitions(workload=workload, plan=matching, scenario=scenario, config_doc=config)
    assert len(calls) == 20
    calls.clear()
    with pytest.raises(ValueError, match="plan"):
        earp_adapter.run_characterization_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(1, ("fresh_store",)), scenario=scenario, config_doc=config)
    assert calls == []


@pytest.mark.parametrize(
    "component,value",
    [
        ("config_sha256", "f" * 64),
        ("query_call", "Engine.search_forged"),
        ("query_params", {"limit": 999, "alpha": 0.7}),
        ("retrieval_mode", "forged"),
        ("max_measurable_k", 999),
        ("use_default_embedder", False),
    ],
)
def test_diagnostic_rechecks_every_normalized_scenario_component(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, component: str, value: object
) -> None:
    from eval.performance import earp_adapter

    root = tmp_path / "experiments"
    run_id, manifest = _quality_graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    plan = earp_adapter.PerformancePlan(20, ("fresh_store",))
    calls: list[object] = []
    from eval.earp.schema.models import RunVerdict
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **kwargs: calls.append(kwargs) or SimpleNamespace(verdict=RunVerdict.COMPLETE, observed_cost={"phases_ms": {"query": 1.0}, "counts": {"queries": 1}}, failure=None, blockers=()))
    baseline = {
        "config_sha256": SHA,
        "query_call": "Engine.search",
        "query_params": {"limit": 10, "alpha": 0.7},
        "retrieval_mode": "text",
        "max_measurable_k": 10,
        "use_default_embedder": True,
    }
    earp_adapter.run_diagnostic_repetitions(workload=workload, plan=plan, scenario=SimpleNamespace(**baseline), config_doc=json.loads(manifest["resolved_config"]["canonical_json"]), experiments_root=root, experiment="review", ts=TS)
    assert len(calls) == 20
    calls.clear()
    normalized = dict(baseline)
    normalized[component] = value
    wrong_scenario = SimpleNamespace(**normalized)
    with pytest.raises(ValueError, match="verified workload"):
        earp_adapter.run_diagnostic_repetitions(workload=workload, plan=plan, scenario=wrong_scenario, config_doc=json.loads(manifest["resolved_config"]["canonical_json"]), experiments_root=root, experiment="review", ts=TS)
    assert calls == []

    # The manifest digest may be internally consistent yet the staged config
    # must still equal its canonical JSON and quality sidecar workload.
    run = root / "runs" / run_id
    (run / "config.resolved.yaml").write_text('{"campaign":"characterization"}\n', encoding="utf-8")
    manifest = json.loads((run / "earp.workload-manifest.v1.json").read_text(encoding="utf-8"))
    manifest["resolved_config"]["sha256"] = _digest(run / "config.resolved.yaml")
    _rewrite_manifest_advertisement(root, run_id, manifest)
    with pytest.raises(ValueError, match="canonical"):
        earp_adapter.load_earp_workload(root, run_id)

    root = tmp_path / "quality-sidecar"
    run_id, _ = _quality_graph(root)
    run = root / "runs" / run_id
    result = json.loads((run / "earp.result.v1.json").read_text(encoding="utf-8"))
    result["scenario"]["effective_knobs"]["limit"] = 999
    (run / "earp.result.v1.json").write_text(json.dumps(result) + "\n", encoding="utf-8")
    manifest = json.loads((run / "earp.workload-manifest.v1.json").read_text(encoding="utf-8"))
    manifest["quality_parent"]["result_sha256"] = _digest(run / "earp.result.v1.json")
    _rewrite_manifest_advertisement(root, run_id, manifest)
    with pytest.raises(ValueError, match="quality"):
        earp_adapter.load_earp_workload(root, run_id)


def test_writer_rejects_a_stale_reference_after_manifest_bytes_change(tmp_path: Path) -> None:
    from eval.performance.earp_adapter import PerformanceCell, PerformancePlan, RunSample, load_earp_workload, write_performance_result

    root = tmp_path / "experiments"
    run_id, manifest = _quality_graph(root)
    workload = load_earp_workload(root, run_id)
    run = root / "runs" / run_id
    changed = json.loads((run / "earp.workload-manifest.v1.json").read_text(encoding="utf-8"))
    changed["quality_parent"]["clean"] = False
    _rewrite_manifest_advertisement(root, run_id, changed)
    plan = PerformancePlan(manifest["performance_plan"]["repetitions"], tuple(manifest["performance_plan"]["treatments"]))
    provenance = {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}
    cells = tuple(PerformanceCell.complete(treatment="fresh_store", repetition=rep, samples=(RunSample("fresh_store", rep, {"query": 1.0}, {"queries": 1}, {"fresh_database": True, "open_write_scope": "fresh_store"}),), execution_provenance=provenance) for rep in range(20))
    with pytest.raises(ValueError, match="verified workload|digest|manifest"):
        write_performance_result(experiments_root=root, experiment="review-tamper", ts=TS, workload=workload, plan=plan, cells=cells)


@pytest.mark.parametrize("edge", ("quality_result_path", "quality_result_digest", "resolved_config_path", "resolved_config_digest"))
def test_loader_rejects_each_invalid_outgoing_artifact_edge_before_admission(
    tmp_path: Path, edge: str
) -> None:
    from eval.performance.earp_adapter import load_earp_workload

    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root)
    run = root / "runs" / run_id
    manifest = json.loads((run / "earp.workload-manifest.v1.json").read_text(encoding="utf-8"))
    if edge == "quality_result_path":
        manifest["quality_parent"]["result_path"] = "other-result.json"
    elif edge == "quality_result_digest":
        manifest["quality_parent"]["result_sha256"] = "f" * 64
    elif edge == "resolved_config_path":
        manifest["resolved_config"]["path"] = "other-config.yaml"
    else:
        manifest["resolved_config"]["sha256"] = "f" * 64
    _rewrite_manifest_advertisement(root, run_id, manifest)
    with pytest.raises(ValueError, match="digest|manifest|quality|config"):
        load_earp_workload(root, run_id)


@pytest.mark.parametrize("component", ("corpus", "gold", "projections", "embedder", "device", "query_knobs"))
def test_characterization_refuses_each_resolved_workload_component_mismatch(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, component: str
) -> None:
    from eval.performance import earp_adapter

    root = tmp_path / "experiments"
    run_id, manifest = _characterization_graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    calls: list[object] = []
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **kwargs: calls.append(kwargs) or SimpleNamespace(blocker=None, observed_cost={"phases_ms": {"query": 1.0}, "counts": {"queries": 1}}))
    scenario = _matching_scenario(manifest)
    config = {key: deepcopy(manifest["workload"][key]) for key in ("corpus", "gold", "projections", "embedder", "device")}
    plan = earp_adapter.PerformancePlan(20, ("fresh_store",))
    earp_adapter.run_characterization_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config)
    assert len(calls) == 20
    calls.clear()
    if component == "query_knobs":
        scenario = SimpleNamespace(config_sha256=SHA, query_call="Engine.search", query_params={"limit": 999})
    else:
        config[component] = {"forged": component}
    with pytest.raises(ValueError, match="verified workload"):
        earp_adapter.run_characterization_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config)
    assert calls == []


def test_typed_unavailable_execution_provenance_persists_without_fabrication(
    tmp_path: Path
) -> None:
    from eval.performance.earp_adapter import PerformancePlan, load_earp_workload, run_repetitions, write_performance_result

    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root)
    workload = load_earp_workload(root, run_id)
    unavailable = {name: {"code": "not_observable", "message": f"{name} unavailable"} for name in ("candidate_sha", "clean", "command", "lockfile_sha256", "toolchain", "device", "fixtures")}
    cells = run_repetitions(workload=workload, plan=PerformancePlan(20, ("fresh_store",)), execution_provenance={"unavailable": unavailable}, execute=lambda *_: (_ for _ in ()).throw(TimeoutError("boom")))
    assert cells[0].status == "invalid"
    assert cells[0].execution_provenance == {"unavailable": unavailable}
    outcome = write_performance_result(experiments_root=root, experiment="review-unavailable", ts=TS, workload=workload, plan=PerformancePlan(20, ("fresh_store",)), cells=cells)
    document = json.loads((outcome.run_dir / "performance.earp.v1.json").read_text(encoding="utf-8"))
    assert document["cells"][0]["execution_provenance"] == {"unavailable": unavailable}


def test_observed_cost_v2_schema_is_total_for_flat_and_arm_documents() -> None:
    from eval.earp.observed_cost import Observation, combine_arm_observations
    from eval.earp.schema.validate import validate
    schema_path = Path(__file__).resolve().parents[2] / "eval/earp/schema/earp.observed-cost.v2.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    checkpoints = (
        "phases_ms.open", "phases_ms.ingest", "phases_ms.query",
        "counts.documents", "counts.queries",
        "storage.database_bytes", "storage.wal_bytes", "storage.shm_bytes",
        "query_samples",
    )
    unavailable = {name: {"code": "not_run", "message": f"{name} unavailable"} for name in checkpoints}
    provenance = {"unavailable": {name: {"code": "not_observable", "message": f"{name} unavailable"} for name in ("candidate_sha", "clean", "command", "lockfile_sha256", "toolchain", "device", "fixtures")}}
    flat = Observation("quality", SHA, {}, {}, {}, query_samples=(), unavailable=unavailable, provenance=provenance).as_document()
    assert validate(flat, schema) == []
    for checkpoint in checkpoints:
        missing = deepcopy(flat)
        del missing["unavailable"][checkpoint]
        assert validate(missing, schema), checkpoint
    arms = combine_arm_observations(config_sha256=SHA, arms={"control": flat})
    assert validate(arms, schema) == []


def test_quality_writer_collides_when_observed_per_query_evidence_changes(tmp_path: Path) -> None:
    from eval.earp.schema.models import RunVerdict
    from eval.earp.writer import write_run

    kwargs = {"experiment": "earp-diagnostic", "ts": TS, "config_doc": {"schema_version": "earp.v1", "campaign": "diagnostic"}, "experiments_root": tmp_path / "experiments", "verdict": RunVerdict.COMPLETE, "read": "review", "metrics": {}, "code": {"git_sha": CANDIDATE, "dirty": False, "branch": "same", "baseline_commit": None}, "env": {"python": "3", "lockfile_sha256": None, "gpu": None, "key_deps": {}}, "corpus": {"source": None, "manifest_sha256": None, "datasets": []}, "seeds": {}, "cost_usd": 0.0}
    provenance = {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}
    observed = {"schema_version": "earp.observed-cost.v2", "scope": "one_run_observation", "phases_ms": {"open": 1.0, "ingest": 2.0, "query": 3.0}, "counts": {"documents": 1, "queries": 1}, "storage": {"database_bytes": 1, "wal_bytes": 0, "shm_bytes": 0}, "query_samples": [{"query_id": "q", "outcome": "complete", "wall_ms": 1.0, "result_count": 1}], "unavailable": {}, "provenance": provenance}
    first = write_run(**kwargs, observed_cost=observed)
    changed = deepcopy(observed)
    changed["query_samples"][0]["wall_ms"] = 9.0
    second = write_run(**kwargs, observed_cost=changed)
    assert first.run_id == second.run_id
    assert second.blocker is not None


def test_writer_refuses_a_plan_that_differs_from_the_advertised_manifest(tmp_path: Path) -> None:
    from eval.performance.earp_adapter import PerformanceCell, PerformancePlan, RunSample, load_earp_workload, write_performance_result

    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root)
    workload = load_earp_workload(root, run_id)
    provenance = {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}
    sample = RunSample("fresh_store", 0, {"query": 1.0}, {"queries": 1}, {"fresh_database": True, "open_write_scope": "fresh_store"})
    cell = PerformanceCell.complete(treatment="fresh_store", repetition=0, samples=(sample,), execution_provenance=provenance)
    with pytest.raises(ValueError, match="plan"):
        write_performance_result(experiments_root=root, experiment="review-plan", ts=TS, workload=workload, plan=PerformancePlan(1, ("fresh_store",)), cells=(cell,))


def test_writer_rejects_semantic_cell_errors(tmp_path: Path) -> None:
    from eval.performance.earp_adapter import PerformanceCell, PerformancePlan, RunSample, load_earp_workload, write_performance_result

    root = tmp_path / "experiments"
    run_id, manifest = _quality_graph(root)
    workload = load_earp_workload(root, run_id)
    provenance = {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}
    plan = PerformancePlan(manifest["performance_plan"]["repetitions"], tuple(manifest["performance_plan"]["treatments"]))
    cells = [PerformanceCell.complete(treatment="fresh_store", repetition=rep, samples=(RunSample("fresh_store", rep, {"query": 1.0}, {"queries": 1}, {"fresh_database": True, "open_write_scope": "fresh_store"}),), execution_provenance=provenance) for rep in range(20)]
    mismatched = PerformanceCell.complete(treatment="fresh_store", repetition=0, samples=(RunSample("fresh_store", 99, {"query": 1.0}, {"queries": 1}, {"fresh_database": True, "open_write_scope": "fresh_store"}),), execution_provenance=provenance)
    cells[0] = mismatched
    with pytest.raises(ValueError, match="raw sample"):
        write_performance_result(experiments_root=root, experiment="review-semantic", ts=TS, workload=workload, plan=plan, cells=cells)
    cells[0] = PerformanceCell.invalid(treatment="fresh_store", repetition=0, raw_samples=(), invalidity={"code": "", "message": 1}, execution_provenance=provenance)
    with pytest.raises(ValueError, match="invalidity"):
        write_performance_result(experiments_root=root, experiment="review-invalidity", ts=TS, workload=workload, plan=plan, cells=cells)


def test_performance_schema_rejects_unknown_nested_cell_field() -> None:
    from eval.earp.schema.validate import validate

    schema_path = Path(__file__).resolve().parents[2] / "eval/performance/schema/performance.earp.v1.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    document = {"schema_version": "performance.earp.v1", "run_id": "review", "scope": "repeated_performance_characterization", "relation": "same_candidate_reexecution", "quality_workload_sha256": SHA, "execution_workload_sha256": SHA, "workload": {"config_sha256": SHA, "query_call": "Engine.search", "effective_knobs": {}}, "plan": {"repetitions": 1, "treatments": ["fresh_store"]}, "parent_manifest": {"path": "runs/quality/earp.workload-manifest.v1.json", "sha256": SHA}, "inputs": {"quality_result": {"path": "earp.result.v1.json", "sha256": SHA}, "resolved_config": {"path": "config.resolved.yaml", "sha256": SHA}}, "cells": [{"treatment": "fresh_store", "repetition": 0, "status": "invalid", "raw_samples": [], "invalidity": {"code": "timeout", "message": "boom"}, "execution_provenance": {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}}], "samples": [], "summary": {}}
    assert validate(document, schema) == []
    document["cells"][0]["unexpected"] = True
    assert validate(document, schema)


@pytest.mark.parametrize("status", ("complete", "invalid"))
def test_default_descriptive_nonclaim_manifest_refuses_plan_mismatch(
    tmp_path: Path, status: str
) -> None:
    """The writer's default nonclaim plan is just as authoritative as an explicit one."""
    from eval.performance.earp_adapter import PerformanceCell, PerformancePlan, RunSample, load_earp_workload, write_performance_result

    root = tmp_path / "experiments"
    run_id, manifest = _quality_graph(root)
    assert manifest["performance_plan"]["command"] == "fathomdb-performance diagnostic"
    workload = load_earp_workload(root, run_id)
    provenance = {"candidate_sha": CANDIDATE, "clean": True, "command": "test", "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "fixtures": {}}
    if status == "complete":
        cells = (PerformanceCell.complete(treatment="fresh_store", repetition=0, samples=(RunSample("fresh_store", 0, {"query": 1.0}, {"queries": 1}, {"fresh_database": True, "open_write_scope": "fresh_store"}),), execution_provenance=provenance),)
    else:
        cells = (PerformanceCell.invalid(treatment="fresh_store", repetition=0, raw_samples=(), invalidity={"code": "timeout", "message": "boom"}, execution_provenance=provenance),)
    with pytest.raises(ValueError, match="plan"):
        write_performance_result(experiments_root=root, experiment="review-default-plan", ts=TS, workload=workload, plan=PerformancePlan(1, ("fresh_store",)), cells=cells)


def test_bridge_provenance_is_authoritative_or_typed_unavailable(monkeypatch: pytest.MonkeyPatch) -> None:
    from eval.performance import earp_adapter

    monkeypatch.setattr(earp_adapter._lib, "git_info", lambda: {"git_sha": "unknown", "dirty": False})
    monkeypatch.setattr(earp_adapter._lib, "env_info", lambda: {"python": "", "lockfile_sha256": None})
    provenance = earp_adapter._bridge_provenance()
    unavailable = provenance.get("unavailable", {})
    for field in ("command", "device", "fixtures", "lockfile_sha256"):
        assert field in provenance or field in unavailable
        if field in provenance:
            assert provenance[field] not in ({}, "", "fathomdb-performance", {"kind": "cpu"})
        else:
            assert unavailable[field]["code"] and unavailable[field]["message"]
    assert "candidate_sha" in unavailable


def test_observed_cost_v2_schema_rejects_empty_arm_container() -> None:
    from eval.earp.schema.validate import validate

    schema_path = Path(__file__).resolve().parents[2] / "eval/earp/schema/earp.observed-cost.v2.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    arms = {"schema_version": "earp.observed-cost.v2", "scope": "one_run_observation", "evidence_family_id": "quality", "config_sha256": SHA, "arms": {}}
    assert validate(arms, schema)


def test_cli_default_descriptive_plan_refuses_override_before_execution(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    from eval.performance import cli

    root = tmp_path / "experiments"
    run_id, _ = _quality_graph(root)
    monkeypatch.setattr(cli, "run_and_write_diagnostic_performance", lambda **_: pytest.fail("must refuse before execution"))
    assert cli.main(["diagnostic", "--experiments-root", str(root), "--quality-run", run_id, "--repetitions", "1", "--treatments", "fresh_store"]) == 2


def test_unavailable_bridge_provenance_makes_raw_sample_cell_invalid_not_complete(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    from eval.performance import earp_adapter
    from eval.earp.schema.models import RunVerdict

    root = tmp_path / "experiments"
    run_id, manifest = _quality_graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    unavailable = {name: {"code": "not_observable", "message": f"{name} unavailable"} for name in ("command", "device", "fixtures")}
    monkeypatch.setattr(earp_adapter, "_bridge_provenance", lambda: {"candidate_sha": CANDIDATE, "clean": True, "lockfile_sha256": SHA, "toolchain": {"python": "3"}, "unavailable": unavailable})
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: SimpleNamespace(verdict=RunVerdict.COMPLETE, observed_cost={"phases_ms": {"open": 1.0}, "counts": {"queries": 1}}, failure=None, blockers=()))
    cells = earp_adapter.run_diagnostic_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(20, ("fresh_store",)), scenario=_matching_scenario(manifest), config_doc=json.loads(manifest["resolved_config"]["canonical_json"]), experiments_root=root, experiment="review", ts=TS)
    assert cells[0].status == "invalid"
    assert cells[0].raw_samples
    assert cells[0].execution_provenance["unavailable"] == unavailable


@pytest.mark.parametrize("bridge", ("diagnostic", "characterization"))
def test_bridges_reject_canonical_config_mismatch_before_execution(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, bridge: str
) -> None:
    from eval.performance import earp_adapter

    root = tmp_path / "experiments"
    graph = _quality_graph if bridge == "diagnostic" else _characterization_graph
    run_id, manifest = graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: pytest.fail("must refuse before execution"))
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **_: pytest.fail("must refuse before execution"))
    scenario = _matching_scenario(manifest)
    config = json.loads(manifest["resolved_config"]["canonical_json"])
    config["forged"] = True
    plan = earp_adapter.PerformancePlan(20, ("fresh_store",))
    with pytest.raises(ValueError, match="config|verified workload"):
        if bridge == "diagnostic":
            earp_adapter.run_diagnostic_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config, experiments_root=root, experiment="review", ts=TS)
        else:
            earp_adapter.run_characterization_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config)


def test_observed_cost_v2_provenance_has_actual_or_unavailable_operational_fields() -> None:
    from eval.earp.observed_cost import Observation

    unavailable = {name: {"code": "not_observable", "message": f"{name} unavailable"} for name in ("command", "lockfile_sha256", "fixtures")}
    document = Observation("quality", SHA, {}, {}, {}, unavailable={"query_samples": {"code": "not_run", "message": "blocked"}}, provenance={"candidate_sha": CANDIDATE, "clean": True, "toolchain": {"python": "3"}, "device": {"kind": "cpu"}, "unavailable": unavailable}).as_document()
    for name in ("command", "lockfile_sha256", "fixtures"):
        assert name in document["provenance"] or document["provenance"]["unavailable"][name]["code"]


@pytest.mark.parametrize("bridge", ("diagnostic", "characterization"))
def test_direct_programmatic_bridges_preserve_unavailable_operational_provenance(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, bridge: str
) -> None:
    from eval.performance import earp_adapter
    from eval.earp.schema.models import RunVerdict

    root = tmp_path / "experiments"
    run_id, manifest = (_quality_graph if bridge == "diagnostic" else _characterization_graph)(root)
    unrelated = tmp_path / "unrelated-cwd"
    unrelated.mkdir()
    monkeypatch.chdir(unrelated)
    monkeypatch.setattr(earp_adapter._lib, "env_info", lambda: {"python": "3.12", "lockfile_sha256": None})
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: SimpleNamespace(verdict=RunVerdict.COMPLETE, observed_cost={"phases_ms": {"open": 1.0}, "counts": {"queries": 1}}, failure=None, blockers=()))
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **_: SimpleNamespace(blocker=None, observed_cost={"phases_ms": {"open": 1.0}, "counts": {"queries": 1}}))
    workload = earp_adapter.load_earp_workload(root, run_id)
    scenario = _matching_scenario(manifest)
    plan = earp_adapter.PerformancePlan(20, ("fresh_store",))
    config = json.loads(manifest["resolved_config"]["canonical_json"])
    if bridge == "diagnostic":
        cells = earp_adapter.run_diagnostic_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config, experiments_root=root, experiment="review", ts=TS)
        outcome = earp_adapter.run_and_write_diagnostic_performance(workload=workload, plan=plan, scenario=scenario, config_doc=config, experiments_root=root, experiment="review-write", ts=TS)
    else:
        cells = earp_adapter.run_characterization_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config)
        outcome = earp_adapter.run_and_write_characterization_performance(workload=workload, plan=plan, scenario=scenario, config_doc=config, experiments_root=root, experiment="review-write", ts=TS)
    assert cells[0].status == "invalid"
    unavailable = cells[0].execution_provenance["unavailable"]
    for field in ("command", "lockfile_sha256", "fixtures"):
        assert unavailable[field]["code"] and unavailable[field]["message"]
    document = json.loads((outcome.run_dir / "performance.earp.v1.json").read_text(encoding="utf-8"))
    assert document["cells"][0]["status"] == "invalid"


@pytest.mark.parametrize("bridge", ("diagnostic", "characterization"))
@pytest.mark.parametrize(
    "component,value",
    (("retrieval_mode", "vector"), ("use_default_embedder", False), ("effective_limit", 999)),
)
def test_descriptive_bridges_reject_retrieval_scenario_drift_before_execution(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, bridge: str, component: str, value: object
) -> None:
    from eval.performance import earp_adapter

    root = tmp_path / "experiments"
    graph = _quality_graph if bridge == "diagnostic" else _characterization_graph
    run_id, manifest = graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: pytest.fail("must reject drift before execution"))
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **_: pytest.fail("must reject drift before execution"))
    scenario_fields = {"config_sha256": SHA, "query_call": "Engine.search", "query_params": dict(manifest["workload"]["effective_knobs"]), "retrieval_mode": "text", "use_default_embedder": True, "max_measurable_k": 10}
    if component == "effective_limit":
        scenario_fields["query_params"]["limit"] = value
        scenario_fields["max_measurable_k"] = value
    else:
        scenario_fields[component] = value
    scenario = SimpleNamespace(**scenario_fields)
    plan = earp_adapter.PerformancePlan(20, ("fresh_store",))
    config = json.loads(manifest["resolved_config"]["canonical_json"])
    with pytest.raises(ValueError, match="scenario|verified workload|input"):
        if bridge == "diagnostic":
            earp_adapter.run_diagnostic_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config, experiments_root=root, experiment="review", ts=TS)
        else:
            earp_adapter.run_characterization_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config)


@pytest.mark.parametrize("bridge", ("diagnostic", "characterization"))
@pytest.mark.parametrize("missing", ("retrieval_mode", "max_measurable_k", "use_default_embedder"))
def test_direct_bridges_refuse_missing_normalized_scenario_field_before_execution(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, bridge: str, missing: str
) -> None:
    from eval.performance import earp_adapter

    root = tmp_path / "experiments"
    graph = _quality_graph if bridge == "diagnostic" else _characterization_graph
    run_id, manifest = graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: pytest.fail("must refuse before execution"))
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **_: pytest.fail("must refuse before execution"))
    fields = {"config_sha256": SHA, "query_call": "Engine.search", "query_params": dict(manifest["workload"]["effective_knobs"]), "retrieval_mode": "text", "max_measurable_k": 10, "use_default_embedder": True}
    del fields[missing]
    scenario = SimpleNamespace(**fields)
    plan = earp_adapter.PerformancePlan(20, ("fresh_store",))
    config = json.loads(manifest["resolved_config"]["canonical_json"])
    with pytest.raises(ValueError, match="scenario|normalized|verified workload"):
        if bridge == "diagnostic":
            earp_adapter.run_diagnostic_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config, experiments_root=root, experiment="review", ts=TS)
        else:
            earp_adapter.run_characterization_repetitions(workload=workload, plan=plan, scenario=scenario, config_doc=config)


@pytest.mark.parametrize("bridge", ("diagnostic", "characterization"))
@pytest.mark.parametrize("missing", ("retrieval_mode", "max_measurable_k", "use_default_embedder"))
def test_missing_normalized_field_refuses_when_canonical_config_is_invalid(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, bridge: str, missing: str
) -> None:
    from eval.performance import earp_adapter

    root = tmp_path / "experiments"
    graph = _quality_graph if bridge == "diagnostic" else _characterization_graph
    run_id, manifest = graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: pytest.fail("must refuse before execution"))
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **_: pytest.fail("must refuse before execution"))
    fields = {"config_sha256": SHA, "query_call": "Engine.search", "query_params": dict(manifest["workload"]["effective_knobs"]), "retrieval_mode": "text", "max_measurable_k": 10, "use_default_embedder": True}
    del fields[missing]
    scenario = SimpleNamespace(**fields)
    with pytest.raises(ValueError, match="canonical config"):
        if bridge == "diagnostic":
            earp_adapter.run_diagnostic_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(20, ("fresh_store",)), scenario=scenario, config_doc={"forged": True}, experiments_root=root, experiment="review", ts=TS)
        else:
            earp_adapter.run_characterization_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(20, ("fresh_store",)), scenario=scenario, config_doc={"forged": True})


@pytest.mark.parametrize("bridge", ("diagnostic", "characterization"))
@pytest.mark.parametrize("missing", ("config_sha256", "query_call", "query_params"))
def test_direct_bridges_refuse_missing_core_scenario_field_before_execution(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, bridge: str, missing: str
) -> None:
    from eval.performance import earp_adapter

    root = tmp_path / "experiments"
    graph = _quality_graph if bridge == "diagnostic" else _characterization_graph
    run_id, manifest = graph(root)
    workload = earp_adapter.load_earp_workload(root, run_id)
    monkeypatch.setattr("eval.earp.runner.run_diagnostic", lambda **_: pytest.fail("must refuse before execution"))
    monkeypatch.setattr("eval.earp.characterize.execute_arm", lambda **_: pytest.fail("must refuse before execution"))
    fields = {"config_sha256": SHA, "query_call": "Engine.search", "query_params": dict(manifest["workload"]["effective_knobs"]), "retrieval_mode": "text", "max_measurable_k": 10, "use_default_embedder": True}
    del fields[missing]
    scenario = SimpleNamespace(**fields)
    config = json.loads(manifest["resolved_config"]["canonical_json"])
    with pytest.raises(ValueError, match="scenario|normalized|verified workload"):
        if bridge == "diagnostic":
            earp_adapter.run_diagnostic_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(20, ("fresh_store",)), scenario=scenario, config_doc=config, experiments_root=root, experiment="review", ts=TS)
        else:
            earp_adapter.run_characterization_repetitions(workload=workload, plan=earp_adapter.PerformancePlan(20, ("fresh_store",)), scenario=scenario, config_doc=config)
