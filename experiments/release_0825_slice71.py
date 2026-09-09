"""Strict local contracts and classifiers for the 0.8.25 Slice 71 evidence."""

from __future__ import annotations

import re
from collections.abc import Mapping
from typing import NoReturn


class Slice71ContractError(ValueError):
    """Raised when a Slice 71 manifest or receipt drifts from its sealed shape."""


_SHA = re.compile(r"[0-9a-f]{40}")
_TOP_KEYS = {
    "schema_version",
    "candidate_ref",
    "baselines",
    "arm_order",
    "workloads",
    "environment_policy",
    "timeouts_s",
    "raw_root",
}
_BASELINE_KEYS = {"ac013_ref", "ingest_ref"}
_WORKLOAD_KEYS = {"ac013", "ingest"}
_AC013_KEYS = {
    "corpus_n",
    "vector_dim",
    "samples",
    "treatment",
    "repetitions_per_arm",
    "p50_budget_ms",
    "p99_budget_ms",
    "max_within_arm_range_percent",
}
_INGEST_KEYS = {
    "corpus_n",
    "batch_size",
    "embedder",
    "repetitions_per_arm",
    "median_regression_limit_percent",
    "max_within_arm_spread_percent",
    "expected_ingest_events",
    "ablation_repetitions",
    "ablation_cells",
}
_ENVIRONMENT_KEYS = {
    "max_load_per_online_cpu",
    "min_available_memory_percent",
    "max_swap_io_delta",
    "thermal_throttled",
    "forbid_competing_processes",
}
_TIMEOUT_KEYS = {"ac013_cell", "ingest_cell"}
_RECEIPT_KEYS = {
    "schema_version",
    "manifest_sha256",
    "candidate_ref",
    "started_at",
    "finished_at",
    "cells",
    "classifications",
    "errors",
}
_CELL_KEYS = {
    "workload",
    "arm",
    "ref",
    "ordinal",
    "started_at",
    "finished_at",
    "process_identity",
    "build_identity",
    "host_identity",
    "fixture_sha256",
    "config_sha256",
    "raw_log_sha256",
    "result_state",
    "error",
    "metrics",
}
_PROCESS_KEYS = {"fresh_process", "test_threads"}
_BUILD_KEYS = {"profile", "rustc", "llvm"}
_HOST_KEYS = {"host", "kernel", "arch", "online_cpus"}
_AC013_METRIC_KEYS = {
    "n",
    "samples",
    "vector_dim",
    "seed_write_ms",
    "projection_drain_ms",
    "p50_ms",
    "p99_ms",
}
_CLASSIFICATION_KEYS = {"ac013", "ingest"}


def _fail(message: str) -> NoReturn:
    raise Slice71ContractError(message)


def _mapping(value: object, path: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        _fail(f"{path} must be an object")
    return value


def _exact_keys(value: Mapping[str, object], expected: set[str], path: str) -> None:
    actual = set(value)
    if actual != expected:
        _fail(f"{path} keys differ: missing={sorted(expected - actual)} extra={sorted(actual - expected)}")


def _expect(value: object, expected: object, path: str) -> None:
    if value != expected or type(value) is not type(expected):
        _fail(f"{path} must be {expected!r}")


def validate_manifest(document: object) -> None:
    """Reject any missing, extra, mistyped, or drifted Slice 71 manifest field."""

    root = _mapping(document, "/")
    _exact_keys(root, _TOP_KEYS, "/")
    _expect(root["schema_version"], "slice71-manifest.v1", "/schema_version")
    if not isinstance(root["candidate_ref"], str) or not _SHA.fullmatch(root["candidate_ref"]):
        _fail("/candidate_ref must be a lowercase 40-hex commit")

    baselines = _mapping(root["baselines"], "/baselines")
    _exact_keys(baselines, _BASELINE_KEYS, "/baselines")
    _expect(
        baselines["ac013_ref"],
        "4fc1b890a11ebfaa8f11b15823656e856002807a",
        "/baselines/ac013_ref",
    )
    _expect(
        baselines["ingest_ref"],
        "b2bfb1f318f58041144acb2356a6a4c9624068b9",
        "/baselines/ingest_ref",
    )
    _expect(root["arm_order"], ["B", "C", "C", "B", "B", "C"], "/arm_order")

    workloads = _mapping(root["workloads"], "/workloads")
    _exact_keys(workloads, _WORKLOAD_KEYS, "/workloads")
    ac013 = _mapping(workloads["ac013"], "/workloads/ac013")
    _exact_keys(ac013, _AC013_KEYS, "/workloads/ac013")
    for key, expected in {
        "corpus_n": 10_000,
        "vector_dim": 384,
        "samples": 1_000,
        "treatment": "warm",
        "repetitions_per_arm": 3,
        "p50_budget_ms": 80,
        "p99_budget_ms": 300,
        "max_within_arm_range_percent": 20,
    }.items():
        _expect(ac013[key], expected, f"/workloads/ac013/{key}")

    ingest = _mapping(workloads["ingest"], "/workloads/ingest")
    _exact_keys(ingest, _INGEST_KEYS, "/workloads/ingest")
    for key, expected in {
        "corpus_n": 10_000,
        "batch_size": 256,
        "embedder": "none",
        "repetitions_per_arm": 3,
        "median_regression_limit_percent": 20,
        "max_within_arm_spread_percent": 25,
        "expected_ingest_events": 30_000,
        "ablation_repetitions": 3,
        "ablation_cells": ["production", "counter_only", "nonce_only", "no_op"],
    }.items():
        _expect(ingest[key], expected, f"/workloads/ingest/{key}")

    environment = _mapping(root["environment_policy"], "/environment_policy")
    _exact_keys(environment, _ENVIRONMENT_KEYS, "/environment_policy")
    for key, expected in {
        "max_load_per_online_cpu": 0.5,
        "min_available_memory_percent": 25,
        "max_swap_io_delta": 0,
        "thermal_throttled": False,
        "forbid_competing_processes": True,
    }.items():
        _expect(environment[key], expected, f"/environment_policy/{key}")

    timeouts = _mapping(root["timeouts_s"], "/timeouts_s")
    _exact_keys(timeouts, _TIMEOUT_KEYS, "/timeouts_s")
    _expect(timeouts["ac013_cell"], 3_600, "/timeouts_s/ac013_cell")
    _expect(timeouts["ingest_cell"], 900, "/timeouts_s/ingest_cell")
    _expect(root["raw_root"], "dev/plans/runs/0.8.25-slice-71", "/raw_root")


def _hex_digest(value: object, length: int, path: str) -> None:
    if not isinstance(value, str) or re.fullmatch(rf"[0-9a-f]{{{length}}}", value) is None:
        _fail(f"{path} must be a lowercase {length}-hex digest")


def _nonempty_string(value: object, path: str) -> None:
    if not isinstance(value, str) or not value:
        _fail(f"{path} must be a non-empty string")


def _nonnegative_number(value: object, path: str) -> None:
    if isinstance(value, bool) or not isinstance(value, int | float) or value < 0:
        _fail(f"{path} must be a non-negative number")


def validate_receipt(document: object, manifest_document: object) -> None:
    """Reject receipt drift and cross-check every retained AC-013 evidence cell."""

    validate_manifest(manifest_document)
    manifest = _mapping(manifest_document, "/manifest")
    root = _mapping(document, "/")
    _exact_keys(root, _RECEIPT_KEYS, "/")
    _expect(root["schema_version"], "slice71-receipt.v1", "/schema_version")
    _hex_digest(root["manifest_sha256"], 64, "/manifest_sha256")
    _expect(root["candidate_ref"], manifest["candidate_ref"], "/candidate_ref")
    _nonempty_string(root["started_at"], "/started_at")
    _nonempty_string(root["finished_at"], "/finished_at")

    cells = root["cells"]
    if not isinstance(cells, list):
        _fail("/cells must be an array")
    expected_arms = manifest["arm_order"]
    if len(cells) != len(expected_arms):  # type: ignore[arg-type]
        _fail("/cells must contain exactly the sealed six AC-013 repetitions")
    baselines = _mapping(manifest["baselines"], "/manifest/baselines")
    workloads = _mapping(manifest["workloads"], "/manifest/workloads")
    ac013 = _mapping(workloads["ac013"], "/manifest/workloads/ac013")
    ordinals = {"B": 0, "C": 0}
    for index, (cell_value, expected_arm) in enumerate(zip(cells, expected_arms, strict=True)):
        path = f"/cells/{index}"
        cell = _mapping(cell_value, path)
        _exact_keys(cell, _CELL_KEYS, path)
        _expect(cell["workload"], "ac013", f"{path}/workload")
        _expect(cell["arm"], expected_arm, f"{path}/arm")
        ordinals[expected_arm] += 1
        _expect(cell["ordinal"], ordinals[expected_arm], f"{path}/ordinal")
        expected_ref = baselines["ac013_ref"] if expected_arm == "B" else manifest["candidate_ref"]
        _expect(cell["ref"], expected_ref, f"{path}/ref")
        _nonempty_string(cell["started_at"], f"{path}/started_at")
        _nonempty_string(cell["finished_at"], f"{path}/finished_at")

        process = _mapping(cell["process_identity"], f"{path}/process_identity")
        _exact_keys(process, _PROCESS_KEYS, f"{path}/process_identity")
        _expect(process["fresh_process"], True, f"{path}/process_identity/fresh_process")
        _expect(process["test_threads"], 1, f"{path}/process_identity/test_threads")

        build = _mapping(cell["build_identity"], f"{path}/build_identity")
        _exact_keys(build, _BUILD_KEYS, f"{path}/build_identity")
        _expect(build["profile"], "release", f"{path}/build_identity/profile")
        _nonempty_string(build["rustc"], f"{path}/build_identity/rustc")
        _nonempty_string(build["llvm"], f"{path}/build_identity/llvm")

        host = _mapping(cell["host_identity"], f"{path}/host_identity")
        _exact_keys(host, _HOST_KEYS, f"{path}/host_identity")
        for key in ("host", "kernel", "arch"):
            _nonempty_string(host[key], f"{path}/host_identity/{key}")
        if isinstance(host["online_cpus"], bool) or not isinstance(host["online_cpus"], int) or host["online_cpus"] < 1:
            _fail(f"{path}/host_identity/online_cpus must be a positive integer")

        for key in ("fixture_sha256", "config_sha256", "raw_log_sha256"):
            _hex_digest(cell[key], 64, f"{path}/{key}")
        _expect(cell["config_sha256"], root["manifest_sha256"], f"{path}/config_sha256")
        if cell["result_state"] not in {"passed", "failed", "insufficient_samples", "environment_invalid"}:
            _fail(f"{path}/result_state is not recognized")
        if cell["error"] is not None and not isinstance(cell["error"], str):
            _fail(f"{path}/error must be a string or null")

        metrics = _mapping(cell["metrics"], f"{path}/metrics")
        _exact_keys(metrics, _AC013_METRIC_KEYS, f"{path}/metrics")
        for key, expected in {
            "n": ac013["corpus_n"],
            "samples": ac013["samples"],
            "vector_dim": ac013["vector_dim"],
        }.items():
            _expect(metrics[key], expected, f"{path}/metrics/{key}")
        for key in _AC013_METRIC_KEYS - {"n", "samples", "vector_dim"}:
            _nonnegative_number(metrics[key], f"{path}/metrics/{key}")

    classifications = _mapping(root["classifications"], "/classifications")
    _exact_keys(classifications, _CLASSIFICATION_KEYS, "/classifications")
    _expect(classifications["ac013"], "environment_invalid", "/classifications/ac013")
    _expect(classifications["ingest"], "blocked_before_execution", "/classifications/ingest")
    errors = root["errors"]
    if not isinstance(errors, list) or not errors or not all(isinstance(item, str) and item for item in errors):
        _fail("/errors must be a non-empty string array")
