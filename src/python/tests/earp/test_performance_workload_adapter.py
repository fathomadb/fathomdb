"""Derive a performance workload from durable EARP evidence, never a duplicate CLI."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from eval.performance.earp_adapter import load_earp_workload


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _quality_artifacts(run: Path) -> None:
    result = run / "earp.result.v1.json"
    result.write_text(
        json.dumps(
            {
                "scenario": {
                    "config_sha256": "a" * 64,
                    "query_call": "Engine.search",
                    "effective_knobs": {"rerank_depth": 20, "alpha": 0.7, "limit": 10},
                },
            }
        )
        + "\n",
        encoding="utf-8",
    )
    resolved = run / "config.resolved.yaml"
    resolved.write_text('{"campaign":"diagnostic"}\n', encoding="utf-8")


def _advertise_manifest(run: Path) -> None:
    """Write a poisoned shared record that advertises, but cannot replace, the manifest."""
    manifest = run / "earp.workload-manifest.v1.json"
    (run / "record.json").write_text(
        json.dumps(
            {
                "code": {"git_sha": "f" * 40},
                "config": {"sha256": "f" * 64},
                "scenario": {"query_call": "Engine.search", "effective_knobs": {"limit": 999}},
                "artifacts": [
                    {"path": "runs/quality-run/earp.result.v1.json", "sha256": _sha256(run / "earp.result.v1.json")},
                    {"path": "runs/quality-run/config.resolved.yaml", "sha256": _sha256(run / "config.resolved.yaml")},
                    {"path": "runs/quality-run/earp.workload-manifest.v1.json", "sha256": _sha256(manifest)},
                ],
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


def _manifest(run: Path) -> None:
    (run / "earp.workload-manifest.v1.json").write_text(
        json.dumps(
            {
                "schema_version": "earp.workload-manifest.v1",
                "quality_parent": {
                    "run_id": "quality-run",
                    "evidence_family_id": "quality-run",
                    "result_path": "earp.result.v1.json",
                    "result_sha256": _sha256(run / "earp.result.v1.json"),
                    "candidate_sha": "b" * 40,
                    "clean": True,
                },
                "resolved_config": {
                    "path": "config.resolved.yaml",
                    "sha256": _sha256(run / "config.resolved.yaml"),
                    "canonical_json": '{"campaign":"diagnostic"}',
                },
                "workload": {
                    "config_sha256": "a" * 64,
                    "query_call": "Engine.search",
                    "effective_knobs": {"rerank_depth": 20, "alpha": 0.7, "limit": 10},
                },
                "performance_plan": {
                    "treatments": ["fresh_store", "fresh_store_warm_query"],
                    "repetitions": 20,
                    "warmup_rule": "declared",
                    "aggregation_rule": "descriptive_empirical_order_statistics",
                    "invalid_result_policy": "typed_cell",
                    "command": "fathomdb-performance diagnostic",
                },
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    _advertise_manifest(run)


def test_adapter_refuses_unmanifested_quality_artifacts(tmp_path: Path) -> None:
    run = tmp_path / "experiments" / "runs" / "quality-run"
    run.mkdir(parents=True)
    _quality_artifacts(run)
    (run / "record.json").write_text(
        json.dumps({"code": {"git_sha": "b" * 40}, "artifacts": []}), encoding="utf-8"
    )

    with pytest.raises(ValueError, match="workload manifest"):
        load_earp_workload(tmp_path / "experiments", "quality-run")


def test_adapter_derives_exact_workload_only_from_verified_manifest(tmp_path: Path) -> None:
    run = tmp_path / "experiments" / "runs" / "quality-run"
    run.mkdir(parents=True)
    _quality_artifacts(run)
    _manifest(run)

    workload = load_earp_workload(tmp_path / "experiments", "quality-run")

    assert workload.parent_run_id == "quality-run"
    assert workload.evidence_family_id == "quality-run"
    assert workload.config_sha256 == "a" * 64
    # The record intentionally contains a different SHA/config/knob. The
    # verified manifest is the only workload authority.
    assert workload.candidate_sha == "b" * 40
    assert workload.effective_knobs == {"rerank_depth": 20, "alpha": 0.7, "limit": 10}


def test_adapter_refuses_a_quality_result_tampered_after_manifest_staging(tmp_path: Path) -> None:
    run = tmp_path / "experiments" / "runs" / "quality-run"
    run.mkdir(parents=True)
    _quality_artifacts(run)
    _manifest(run)
    (run / "earp.result.v1.json").write_text(
        json.dumps(
            {"scenario": {"config_sha256": "a" * 64, "query_call": "Engine.search_text_only"}}
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="digest"):
        load_earp_workload(tmp_path / "experiments", "quality-run")


def test_adapter_refuses_a_resolved_config_tampered_after_manifest_staging(tmp_path: Path) -> None:
    run = tmp_path / "experiments" / "runs" / "quality-run"
    run.mkdir(parents=True)
    _quality_artifacts(run)
    _manifest(run)
    (run / "config.resolved.yaml").write_text('{"campaign":"poisoned"}\n', encoding="utf-8")

    with pytest.raises(ValueError, match="resolved config.*digest"):
        load_earp_workload(tmp_path / "experiments", "quality-run")


def test_adapter_refuses_an_unrecognized_manifest_schema_even_when_record_advertises_it(
    tmp_path: Path,
) -> None:
    run = tmp_path / "experiments" / "runs" / "quality-run"
    run.mkdir(parents=True)
    _quality_artifacts(run)
    _manifest(run)
    manifest_path = run / "earp.workload-manifest.v1.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["schema_version"] = "earp.workload-manifest.v0"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

    with pytest.raises(ValueError, match="manifest schema"):
        load_earp_workload(tmp_path / "experiments", "quality-run")


def test_adapter_refuses_quality_artifacts_without_candidate_provenance(tmp_path: Path) -> None:
    run = tmp_path / "experiments" / "runs" / "quality-run"
    run.mkdir(parents=True)
    _quality_artifacts(run)
    _manifest(run)
    manifest_path = run / "earp.workload-manifest.v1.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["quality_parent"]["candidate_sha"] = ""
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

    with pytest.raises(ValueError, match="candidate.*provenance"):
        load_earp_workload(tmp_path / "experiments", "quality-run")
