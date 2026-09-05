from __future__ import annotations

import copy

import pytest

from experiments import scale_02_slice40


def config() -> dict[str, object]:
    return {
        "schema_version": "scale-02-slice40.v1",
        "program_track": "SCALE-02",
        "release": "0.8.25",
        "approval": {
            "state": "approved",
            "approved_by": "HITL",
            "approved_at": "2026-09-05",
        },
        "baseline": {
            "source_commit": "0aff1cb08c61a8bb2a004813bbd5604b6ff1a403",
        },
        "candidate": {"source_commit": "a" * 40},
        "workload": {
            "records": 10_000,
            "batch_size": 128,
            "expected_receipts": 79,
            "repetitions": 5,
            "status_records": 50_000,
            "status_warmups": 100,
            "status_samples": 1_000,
            "open_warmups": 5,
            "open_samples": 20,
            "bootstrap_seed": "0x5CA1E025400001",
            "bootstrap_resamples": 2_000,
        },
        "policy": {
            "confidence": 0.95,
            "write_max_relative_regression": 0.03,
            "open_max_relative_regression": 0.10,
            "open_max_absolute_regression_ms": 25.0,
            "status_p95_ms": 5.0,
            "status_p99_ms": 10.0,
            "storage_max_increase_bytes": 65_536,
            "cpu_cuda_status_p95_max_difference_ms": 2.0,
        },
        "artifact_root": "/home/coreyt/projects/fathomdb/data/performance-benchmarking/scale-02/slice40-runs",
        "claim_boundary": "slice40_projection_generation_readiness",
    }


def test_config_accepts_only_the_registered_contract() -> None:
    document = config()
    assert scale_02_slice40.resolve_config(document, validate_repository=False) == document

    unknown = copy.deepcopy(document)
    unknown["surprise"] = True
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="unknown"):
        scale_02_slice40.resolve_config(unknown, validate_repository=False)

    drifted = copy.deepcopy(document)
    drifted["workload"]["repetitions"] = 4  # type: ignore[index]
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="workload"):
        scale_02_slice40.resolve_config(drifted, validate_repository=False)


def test_write_and_open_verdicts_are_bound_to_registered_limits() -> None:
    passing = scale_02_slice40.comparison_verdict(
        write_upper={"p50": 0.02, "p95": 0.025},
        open_upper_relative=0.09,
        open_absolute_regression_ms=30.0,
        storage_increase_bytes=60_000,
        policy=config()["policy"],  # type: ignore[arg-type]
    )
    assert passing == "pass"

    assert (
        scale_02_slice40.comparison_verdict(
            write_upper={"p50": 0.02, "p95": 0.031},
            open_upper_relative=0.0,
            open_absolute_regression_ms=0.0,
            storage_increase_bytes=0,
            policy=config()["policy"],  # type: ignore[arg-type]
        )
        == "fail"
    )
    assert (
        scale_02_slice40.comparison_verdict(
            write_upper={"p50": 0.0, "p95": 0.0},
            open_upper_relative=0.11,
            open_absolute_regression_ms=26.0,
            storage_increase_bytes=0,
            policy=config()["policy"],  # type: ignore[arg-type]
        )
        == "fail"
    )


def test_paired_relative_upper_is_deterministic() -> None:
    baseline = [10.0, 10.1, 9.9, 10.2, 9.8]
    candidate = [10.1, 10.2, 10.0, 10.3, 9.9]
    first = scale_02_slice40.paired_relative_upper(
        baseline, candidate, seed=41, resamples=2_000
    )
    second = scale_02_slice40.paired_relative_upper(
        baseline, candidate, seed=41, resamples=2_000
    )
    assert first == second
    assert 0.0 < first < 0.03


def test_worker_result_rejects_wrong_counts() -> None:
    result = {
        "records": 10_000,
        "batch_size": 128,
        "receipt_count": 79,
        "operation_count": 10_000,
        "pending_receipt_count": 79,
        "pending_cursor_count": 10_000,
        "errors": 0,
        "timeouts": 0,
    }
    scale_02_slice40.validate_worker_counts(result, config()["workload"])  # type: ignore[arg-type]
    result["pending_receipt_count"] = 78
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="counts"):
        scale_02_slice40.validate_worker_counts(result, config()["workload"])  # type: ignore[arg-type]


def status_result(*, device: str, generation_p95: float = 0.02) -> dict[str, object]:
    return {
        "schema_version": "scale-02-slice40-status.v1",
        "device": device,
        "records": 50_000,
        "warmups": 100,
        "samples": 1_000,
        "errors": 0,
        "timeouts": 0,
        "cold_generation_ms": 75.0,
        "cold_mutation_ms": 75.0,
        "transition_generation_ms": 80.0,
        "generation_p95_ms": generation_p95,
        "generation_p99_ms": 0.03,
        "mutation_p95_ms": 0.02,
        "mutation_p99_ms": 0.03,
        "steady_full_owner_scans": 0,
    }


def test_status_verdict_requires_five_exact_cpu_cuda_repetitions() -> None:
    cpu = [status_result(device="cpu") for _ in range(5)]
    cuda = [status_result(device="cuda:0", generation_p95=0.03) for _ in range(5)]
    assert (
        scale_02_slice40.status_verdict(
            cpu, cuda, config()["workload"], config()["policy"]  # type: ignore[arg-type]
        )
        == "pass"
    )

    cuda[4]["generation_p95_ms"] = 2.03
    assert (
        scale_02_slice40.status_verdict(
            cpu, cuda, config()["workload"], config()["policy"]  # type: ignore[arg-type]
        )
        == "fail"
    )

    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="repetitions"):
        scale_02_slice40.status_verdict(
            cpu[:4], cuda, config()["workload"], config()["policy"]  # type: ignore[arg-type]
        )


def test_parse_status_result_rejects_missing_or_extra_fields() -> None:
    result = status_result(device="cpu")
    assert scale_02_slice40.parse_status_result(result) == result

    missing = dict(result)
    del missing["timeouts"]
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="status result"):
        scale_02_slice40.parse_status_result(missing)

    extra = {**result, "unregistered": True}
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="status result"):
        scale_02_slice40.parse_status_result(extra)
