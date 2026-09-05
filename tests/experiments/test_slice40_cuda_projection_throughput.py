"""Contract tests for the preregistered Slice 40 CUDA throughput witness."""

from __future__ import annotations

import copy

import pytest

from experiments import slice40_cuda_projection_throughput as throughput


def _config() -> dict[str, object]:
    return {
        "schema_version": "scale-02-slice40-cuda-throughput.v1",
        "program_track": "SCALE-02",
        "release": "0.8.25",
        "approval": {
            "state": "approved",
            "approved_by": "HITL",
            "approved_at": "2026-09-05",
        },
        "candidate": {
            "product_commit": "2313fd34ea5ca68346a468d5bba58ba245306c08",
            "wheel_sha256": "a" * 64,
        },
        "execution": {
            "runner_sha256": "b" * 64,
            "measurement_plan_sha256": "c" * 64,
        },
        "workload": {
            "records": 1024,
            "write_batch_size": 128,
            "repetitions": 5,
            "warmup_records": 64,
            "drain_timeout_seconds": 900,
        },
        "runtime": {
            "embed_device": "cuda:0",
            "cuda_uuid": "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b",
            "embedder": "fathomdb-bge-small-en-v1.5",
            "network": "none",
        },
        "policy": {
            "classification": "descriptive",
            "decision_threshold": None,
            "excluded_time": ["artifact_install", "model_load", "model_warmup"],
            "included_time": ["configure_projection", "projection_backfill", "drain"],
        },
        "artifact_root": (
            "/home/coreyt/projects/fathomdb/data/performance-benchmarking/"
            "scale-02/slice40-runs"
        ),
        "claim_boundary": "cuda_projection_backfill_throughput_descriptive",
    }


def test_config_is_closed_and_hash_bound() -> None:
    resolved = throughput.resolve_config(_config(), validate_repository=False)
    assert resolved["workload"]["records"] == 1024

    drift = copy.deepcopy(_config())
    drift["workload"]["repetitions"] = 3  # type: ignore[index]
    with pytest.raises(throughput.ThroughputContractError, match="workload"):
        throughput.resolve_config(drift, validate_repository=False)

    unknown = copy.deepcopy(_config())
    unknown["policy"]["percentile"] = 95  # type: ignore[index]
    with pytest.raises(throughput.ThroughputContractError, match="policy keys"):
        throughput.resolve_config(unknown, validate_repository=False)


def test_summary_rejects_non_cuda_or_incomplete_projection() -> None:
    observations = [
        {
            "repetition": index,
            "elapsed_seconds": 2.0,
            "records": 1024,
            "records_per_second": 512.0,
            "selected_device": "cuda:0",
            "selected_cuda_uuid": "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b",
            "vector_rows": 1024,
            "readiness": "ready",
        }
        for index in range(1, 6)
    ]
    summary = throughput.summarize_observations(observations, _config())
    assert summary == {
        "classification": "descriptive",
        "repetitions": 5,
        "records_per_repetition": 1024,
        "throughput_records_per_second": {
            "minimum": 512.0,
            "median": 512.0,
            "maximum": 512.0,
        },
        "verdict": "observed",
    }

    observations[2]["selected_device"] = "cpu"
    with pytest.raises(throughput.ThroughputContractError, match="cuda:0"):
        throughput.summarize_observations(observations, _config())

    observations[2]["selected_device"] = "cuda:0"
    observations[2]["vector_rows"] = 1023
    with pytest.raises(throughput.ThroughputContractError, match="vector_rows"):
        throughput.summarize_observations(observations, _config())
