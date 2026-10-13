"""Synthetic, no-driver acceptance fixtures for the SDK scoreboard."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments import _lib, sdk_scoreboard


def test_retrieval_timing_is_optional_when_suite_also_requests_latency():
    plan = {"output": {"metrics": ["recall_at_10", "p50_ms"]}}
    case = {"family": "retrieval"}
    assert sdk_scoreboard._required_metrics(plan, case) == {"recall_at_10"}


def test_optional_capability_miss_does_not_fail_required_suite():
    assert sdk_scoreboard._suite_status(
        [(True, "complete", None), (False, "blocked", "unsupported_operation")]
    ) == ("complete", 1)
    assert sdk_scoreboard._suite_status(
        [(True, "complete", None), (True, "blocked", "unsupported_operation")]
    ) == ("blocked", 0)


def test_scale_tier_is_part_of_safe_comparison_projection():
    case = {
        "id": "latency", "family": "latency_capacity", "operation_id": "read.get",
        "operation_version": 1, "scale_tier": "embedded-10k",
        "arguments": {"view": {"valid_as_of": 42}},
    }
    plan = {
        "program_track": None, "suite_id": "suite",
        "fixtures": {"manifest": {"sha256": "a" * 64}},
        "arms": [{"sdk": "python", "artifact": {"sha256": "b" * 64}}],
        "sampling": {"repetitions": 2},
    }
    arm = {
        "sdk": "python", "engine_revision": "abc",
        "dependency_closure_sha256": "c" * 64,
        "native": {"sha256": "d" * 64}, "driver": {"sha256": "e" * 64},
    }
    execution = {
        "schema_version": "sdk.execution-config.v1", "plan": plan,
        "arm": arm, "case": case,
        "sampling": {"repetitions": 2, "timeout_ms": 30000},
    }
    assert sdk_scoreboard._projection(execution, "f" * 64)["scale_tier"] == "embedded-10k"


def _write(path: Path, value: object) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, sort_keys=True, indent=2) + "\n", encoding="utf-8"
    )
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _fixture(tmp_path: Path) -> tuple[Path, Path, Path]:
    plan = {
        "schema_version": "sdk.experiment.v1",
        "suite_id": "sample",
        "program_track": None,
        "fixtures": {"manifest": {"path": "/private/gold", "sha256": "a" * 64}},
        "arms": [
            {"sdk": "python", "artifact": {"path": "/private/arm", "sha256": "b" * 64}}
        ],
        "cases": [
            {
                "id": "search",
                "family": "retrieval",
                "operation_id": "engine.search_text_only",
                "operation_version": 1,
                "required": True,
                "arguments": {
                    "query_set": "all",
                    "limit": 10,
                    "view": {"valid_as_of": 42},
                },
                "setup": "canonical-nodes-v1",
                "boundary": "sdk_call_to_materialized_result",
            }
        ],
        "sampling": {
            "warmup_per_query": 0,
            "repetitions": 2,
            "seed": 1,
            "cache_state": "steady_after_declared_warmup",
            "concurrency": 1,
            "order": "balanced",
        },
        "output": {"result_schema": "sdk.result.v1", "metrics": ["recall_at_10"]},
    }
    plan_path = tmp_path / "plan.json"
    plan_sha = _write(plan_path, plan)
    root = tmp_path / "external"
    receipt_root = tmp_path / "receipts"
    cell = root / "search-python"
    arm = {
        "sdk": "python",
        "sdk_version": "0.1",
        "engine_revision": "abc",
        "dependency_closure_sha256": "c" * 64,
        "native": {"path": "/private/native", "sha256": "d" * 64},
        "driver": {"path": "/private/driver", "sha256": "e" * 64},
    }
    config = {
        "schema_version": "sdk.receipt-config.v1",
        "program_track": None,
        "suite_id": "sample",
        "suite_config_sha256": plan_sha,
        "case_id": "search",
        "family": "retrieval",
        "operation_id": "engine.search_text_only",
        "operation_version": 1,
        "sdk": "python",
        "arguments": plan["cases"][0]["arguments"],
        "sampling": {**plan["sampling"], "timeout_ms": 30000},
        "fixture_sha256": "a" * 64,
        "arm_manifest_sha256": "b" * 64,
        "engine_revision": "abc",
        "dependency_closure_sha256": "c" * 64,
        "native_sha256": "d" * 64,
        "driver_sha256": "e" * 64,
    }
    execution = {
        "schema_version": "sdk.execution-config.v1",
        "plan": plan,
        "arm": arm,
        "case": plan["cases"][0],
        "sampling": config["sampling"],
    }
    execution_sha = _write(cell / "execution-config.json", execution)
    evidence_sha = _write(
        cell / "evidence.json",
        {
            "schema_version": "sdk.retrieval-evidence.v1",
            "suite_attempt_id": "attempt",
            "case_id": "search",
            "sdk": "python",
            "questions": [],
        },
    )
    result = {
        "schema_version": "sdk.result.v1",
        "suite_id": "sample",
        "suite_attempt_id": "attempt",
        "case_id": "search",
        "sdk": "python",
        "axis": "sdk",
        "family": "retrieval",
        "producer": "driver",
        "operation_id": "engine.search_text_only",
        "operation_version": 1,
        "status": "complete",
        "reason": None,
        "metrics": {"recall_at_10": 1.0},
        "ordered_ids": {},
        "evidence_sha256": evidence_sha,
        "samples": [{"query_id": "q", "elapsed_ns": 1, "ids": []}],
        "repetitions": [],
        "setup_witnesses": [],
    }
    result_sha = _write(cell / "result.json", result)
    witness = {
        "schema_version": "sdk.execution.v1",
        "suite_attempt_id": "attempt",
        "case_id": "search",
        "sdk": "python",
        "execution_config_sha256": execution_sha,
        "receipt_config_sha256": _lib.config_sha256(config),
        "result_sha256": result_sha,
        "status": "complete",
        "result_locator": str(cell / "result.json"),
        "host": {"system": "Test"},
    }
    witness_sha = _write(cell / "execution.json", witness)
    run_id = (
        "sdk-sample-search-python-attempt-20261013T1200Z-"
        + _lib.config_sha256(config)[:8]
    )
    record = {
        "schema_version": "experiments.record.v1",
        "run_id": run_id,
        "experiment": "sdk-sample-search-python-attempt",
        "verdict": "complete",
        "config": {
            "sha256": _lib.config_sha256(config),
            "path": None,
            "resolved": config,
        },
        "metrics": {
            "status": "complete",
            "recall_at_10": 1.0,
            "result_sha256": result_sha,
            "execution_witness_sha256": witness_sha,
        },
        "artifacts": [
            {"id": "search-python:result", "sha256": result_sha},
            {"id": "search-python:execution", "sha256": witness_sha},
        ],
    }
    _write(receipt_root / "runs" / run_id / "record.json", record)
    (receipt_root / "runs" / run_id / "config.resolved.yaml").write_text(
        json.dumps(config), encoding="utf-8"
    )
    _write(receipt_root / "runs" / run_id / "metrics.json", record["metrics"])
    receipt_root.joinpath("index.jsonl").write_text(
        json.dumps(
            {
                "schema_version": "experiments.index-row.v1",
                "run_id": run_id,
                "repo": "fathomdb",
                "experiment": record["experiment"],
                "config_sha256": record["config"]["sha256"],
                "verdict": "complete",
                "headline": {"recall_at_10": 1.0},
                "n": 1,
            }
        )
        + "\n",
        encoding="utf-8",
    )
    _write(
        root / "suite-manifest.json",
        {
            "schema_version": "sdk.suite-manifest.v1",
            "suite_id": "sample",
            "suite_attempt_id": "attempt",
            "suite_config_sha256": plan_sha,
            "expected_cells": ["search-python"],
            "cells": [
                {
                    "case_id": "search",
                    "sdk": "python",
                    "run_id": run_id,
                    "status": "complete",
                    "result_sha256": result_sha,
                    "witness_sha256": witness_sha,
                }
            ],
        },
    )
    return plan_path, root / "suite-manifest.json", receipt_root


def test_complete_cell_and_three_path_free_family_views(tmp_path: Path) -> None:
    report = sdk_scoreboard.build_scoreboard(*_fixture(tmp_path))
    assert report["suite_status"] == "complete"
    assert set(report["families"]) == {"latency_capacity", "retrieval", "answer"}
    row = report["families"]["retrieval"]["sdk"][0]
    assert row["status"] == "complete" and row["metrics"] == {"recall_at_10": 1.0}
    assert row["evidence_status"] == "local_descriptive"
    assert report["families"]["retrieval"]["product"] == []
    assert "/private/" not in json.dumps(report)
    assert "delta" not in json.dumps(report)


@pytest.mark.parametrize("required", ["false", 0, None])
def test_malformed_required_flag_refuses_scoreboard(
    tmp_path: Path, required: object
) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    value = json.loads(plan.read_text(encoding="utf-8"))
    if required is None:
        del value["cases"][0]["required"]
    else:
        value["cases"][0]["required"] = required
    _write(plan, value)
    with pytest.raises(ValueError, match="case required"):
        sdk_scoreboard.build_scoreboard(plan, manifest, receipts)


@pytest.mark.parametrize("roster", ["cases", "arms"])
def test_empty_roster_refuses_scoreboard(tmp_path: Path, roster: str) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    value = json.loads(plan.read_text(encoding="utf-8"))
    value[roster] = []
    _write(plan, value)
    with pytest.raises(ValueError, match="plan roster"):
        sdk_scoreboard.build_scoreboard(plan, manifest, receipts)


def test_missing_cell_stays_in_denominator(tmp_path: Path) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    data = json.loads(manifest.read_text())
    data["cells"] = []
    _write(manifest, data)
    report = sdk_scoreboard.build_scoreboard(plan, manifest, receipts)
    assert report["suite_status"] == "incomplete"
    assert report["families"]["retrieval"]["sdk"][0]["status"] == "incomplete"


def test_manifest_cannot_shrink_expected_roster(tmp_path: Path) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    data = json.loads(manifest.read_text())
    data["expected_cells"] = []
    _write(manifest, data)
    report = sdk_scoreboard.build_scoreboard(plan, manifest, receipts)
    assert report["suite_status"] == "invalid"
    assert report["expected_cells"] == 1
    assert report["families"]["retrieval"]["sdk"][0]["status"] == "invalid"


def test_duplicate_index_row_is_invalid(tmp_path: Path) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    index = receipts / "index.jsonl"
    index.write_text(index.read_text() * 2, encoding="utf-8")
    report = sdk_scoreboard.build_scoreboard(plan, manifest, receipts)
    assert report["families"]["retrieval"]["sdk"][0]["status"] == "invalid"


def test_path_bearing_receipt_is_invalid(tmp_path: Path) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    run_id = json.loads(manifest.read_text())["cells"][0]["run_id"]
    record_path = receipts / "runs" / run_id / "record.json"
    record = json.loads(record_path.read_text())
    record["read"] = "/private/unexpected"
    _write(record_path, record)
    report = sdk_scoreboard.build_scoreboard(plan, manifest, receipts)
    assert report["families"]["retrieval"]["sdk"][0]["status"] == "invalid"


def test_malformed_metrics_is_invalid_instead_of_crashing(tmp_path: Path) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    run_id = json.loads(manifest.read_text())["cells"][0]["run_id"]
    record_path = receipts / "runs" / run_id / "record.json"
    record = json.loads(record_path.read_text())
    record["metrics"] = []
    _write(record_path, record)
    report = sdk_scoreboard.build_scoreboard(plan, manifest, receipts)
    assert report["families"]["retrieval"]["sdk"][0]["status"] == "invalid"


def test_unsafe_reason_is_not_rendered(tmp_path: Path) -> None:
    assert sdk_scoreboard._safe_reason("/private/secret-api-key") == "invalid_reason"


def test_product_receipt_uses_separate_axis_and_legacy_label(
    tmp_path: Path, monkeypatch
) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    product_record = tmp_path / "product-record.json"
    product_record.write_text("{}", encoding="utf-8")
    monkeypatch.setattr(
        sdk_scoreboard.competitive_product,
        "validate_receipt",
        lambda *_args: {
            "run_id": "product-run",
            "axis": "product",
            "family": "retrieval",
            "status": "complete",
            "evidence_status": "legacy_path_bearing",
            "metrics": {"recall_at_10": 0.7},
            "comparability": "context_only",
            "product": {"name": "example", "version": "1", "deployment": "embedded"},
        },
    )
    report = sdk_scoreboard.build_scoreboard(
        plan, manifest, receipts, product_sources=[(product_record, None)]
    )
    assert (
        report["families"]["retrieval"]["product"][0]["evidence_status"]
        == "legacy_path_bearing"
    )
    assert len(report["families"]["retrieval"]["sdk"]) == 1
    assert "delta" not in json.dumps(report)


def test_blocked_cell_has_no_performance_metrics(tmp_path: Path) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    data = json.loads(manifest.read_text())
    entry = data["cells"][0]
    entry["status"] = "blocked"
    run_id = entry["run_id"]
    cell_root = manifest.parent / "search-python"
    result = json.loads((cell_root / "result.json").read_text())
    result.update(
        status="blocked", reason="unsupported_operation", metrics={}, samples=[]
    )
    entry["result_sha256"] = _write(cell_root / "result.json", result)
    witness = json.loads((cell_root / "execution.json").read_text())
    witness.update(status="blocked", result_sha256=entry["result_sha256"])
    entry["witness_sha256"] = _write(cell_root / "execution.json", witness)
    record_path = receipts / "runs" / run_id / "record.json"
    record = json.loads(record_path.read_text())
    record["verdict"] = "blocked"
    record["metrics"] = {
        "status": "blocked",
        "result_sha256": entry["result_sha256"],
        "execution_witness_sha256": entry["witness_sha256"],
    }
    record["artifacts"] = [
        {"id": "search-python:result", "sha256": entry["result_sha256"]},
        {"id": "search-python:execution", "sha256": entry["witness_sha256"]},
    ]
    _write(record_path, record)
    _write(record_path.parent / "metrics.json", record["metrics"])
    index = receipts / "index.jsonl"
    index_row = json.loads(index.read_text())
    index_row.update(verdict="blocked", headline={}, n=0)
    index.write_text(json.dumps(index_row) + "\n", encoding="utf-8")
    _write(manifest, data)
    report = sdk_scoreboard.build_scoreboard(plan, manifest, receipts)
    row = report["families"]["retrieval"]["sdk"][0]
    assert report["suite_status"] == "blocked"
    assert row["status"] == "blocked" and row["metrics"] == {}


@pytest.mark.parametrize(
    "target", ["result", "witness", "receipt", "index", "execution", "config"]
)
def test_tampered_link_is_invalid(tmp_path: Path, target: str) -> None:
    plan, manifest, receipts = _fixture(tmp_path)
    data = json.loads(manifest.read_text())
    run_id = data["cells"][0]["run_id"]
    paths = {
        "result": manifest.parent / "search-python" / "result.json",
        "witness": manifest.parent / "search-python" / "execution.json",
        "execution": manifest.parent / "search-python" / "execution-config.json",
        "receipt": receipts / "runs" / run_id / "record.json",
        "index": receipts / "index.jsonl",
        "config": receipts / "runs" / run_id / "config.resolved.yaml",
    }
    if target == "config":
        value = json.loads(paths[target].read_text())
        value["fixture_sha256"] = "f" * 64
        _write(paths[target], value)
    elif target == "index":
        value = json.loads(paths[target].read_text())
        value["config_sha256"] = "f" * 64
        paths[target].write_text(json.dumps(value) + "\n", encoding="utf-8")
    elif target == "receipt":
        value = json.loads(paths[target].read_text())
        value["metrics"]["result_sha256"] = "f" * 64
        _write(paths[target], value)
    else:
        value = json.loads(paths[target].read_text())
        value["tampered"] = True
        _write(paths[target], value)
    report = sdk_scoreboard.build_scoreboard(plan, manifest, receipts)
    assert report["suite_status"] == "invalid"
    assert report["families"]["retrieval"]["sdk"][0]["status"] == "invalid"
