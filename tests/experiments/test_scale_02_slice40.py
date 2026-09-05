from __future__ import annotations

import copy
import sqlite3

import pytest

from experiments import scale_02_slice40


def config() -> dict[str, object]:
    return {
        "schema_version": "scale-02-slice40.v3",
        "program_track": "SCALE-02",
        "release": "0.8.25",
        "campaign": "common",
        "approval": {
            "state": "approved",
            "approved_by": "HITL",
            "approved_at": "2026-09-05",
        },
        "baseline": {
            "source_commit": "0aff1cb08c61a8bb2a004813bbd5604b6ff1a403",
        },
        "candidate": {"source_commit": "a" * 40},
        "execution": {
            "runner_sha256": "b" * 64,
            "record_library_sha256": "c" * 64,
            "statistics_library_sha256": "d" * 64,
            "common_worker_sha256": "e" * 64,
            "status_test_sha256": "f" * 64,
        },
        "measurement_plan": {
            "path": "experiments/configs/scale-02/slice40-common-measurement-plan.v1.json",
            "sha256": "1" * 64,
            "plan_id": "slice40-common-v1",
        },
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
    assert (
        scale_02_slice40.resolve_config(document, validate_repository=False) == document
    )

    unknown = copy.deepcopy(document)
    unknown["surprise"] = True
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="unknown"):
        scale_02_slice40.resolve_config(unknown, validate_repository=False)

    drifted = copy.deepcopy(document)
    drifted["workload"]["repetitions"] = 4  # type: ignore[index]
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="workload"):
        scale_02_slice40.resolve_config(drifted, validate_repository=False)

    wrong_campaign = copy.deepcopy(document)
    wrong_campaign["campaign"] = "unknown"
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="campaign"):
        scale_02_slice40.resolve_config(wrong_campaign, validate_repository=False)


def test_decision_metrics_are_compact_and_campaign_specific() -> None:
    common = scale_02_slice40.decision_metrics(
        "common",
        {
            "verdict": "pass",
            "write_upper_95_relative_regression": {"p50": 0.01, "p95": 0.02},
            "open_upper_95_relative_regression": 0.03,
            "open_mean_absolute_regression_ms": 1.25,
            "maximum_paired_storage_increase_bytes": 4096,
        },
    )
    assert common == {
        "schema_version": "scale-02-slice40-decision.v1",
        "campaign": "common",
        "verdict": "pass",
        "write_p50_upper_relative_regression": 0.01,
        "write_p95_upper_relative_regression": 0.02,
        "open_upper_relative_regression": 0.03,
        "open_mean_absolute_regression_ms": 1.25,
        "maximum_paired_storage_increase_bytes": 4096,
    }

    status = scale_02_slice40.decision_metrics(
        "status",
        {
            "verdict": "pass",
            "maximum_cpu_cuda_p95_difference_ms": 0.4,
            "status_bounds": {
                "maximum_generation_p95_ms": 0.1,
                "maximum_generation_p99_ms": 0.2,
                "maximum_mutation_p95_ms": 0.3,
                "maximum_mutation_p99_ms": 0.4,
                "maximum_steady_full_owner_scans": 0,
                "total_errors": 0,
                "total_timeouts": 0,
            },
        },
    )
    assert status["campaign"] == "status"
    assert status["maximum_generation_p99_ms"] == 0.2
    assert status["maximum_steady_full_owner_scans"] == 0


def test_record_registration_requires_classification_before_index(
    monkeypatch, tmp_path
) -> None:
    order: list[str] = []

    def write_record(*args, **kwargs):
        before_index = kwargs["before_index"]
        run_dir = tmp_path / "runs" / "run"
        run_dir.mkdir(parents=True)
        (run_dir / "record.json").write_text("{}\n", encoding="utf-8")
        (run_dir / "metrics.json").write_text("{}\n", encoding="utf-8")
        before_index("run", run_dir)
        order.append("index")
        return "run", run_dir

    monkeypatch.setattr(scale_02_slice40._lib, "write_record", write_record)
    monkeypatch.setattr(
        scale_02_slice40,
        "_write_run_classification",
        lambda **kwargs: order.append("classification"),
    )
    monkeypatch.setattr(
        scale_02_slice40,
        "_repository_locator",
        lambda path, label: str(path),
    )
    scale_02_slice40._register_run(
        experiment="fixture",
        config=config(),
        config_path=tmp_path / "config.json",
        decision_metrics={"verdict": "pass"},
        detail={"raw": 1},
        record_arguments={},
        code_git_sha="a" * 40,
    )
    assert order == ["classification", "index"]
    assert (tmp_path / "runs" / "run" / "detail.json").is_file()


def test_execution_preflight_rejects_dirty_source(monkeypatch, tmp_path) -> None:
    files = {}
    execution = {}
    for index, label in enumerate(
        (
            "runner_sha256",
            "record_library_sha256",
            "statistics_library_sha256",
            "common_worker_sha256",
            "status_test_sha256",
        )
    ):
        path = tmp_path / f"source-{index}"
        path.write_text(label, encoding="utf-8")
        files[label] = path
        execution[label] = scale_02_slice40._sha256(path)
    monkeypatch.setattr(scale_02_slice40, "_EXECUTION_FILES", files)
    monkeypatch.setattr(
        scale_02_slice40._lib,
        "git_info",
        lambda: {"git_sha": "a" * 40, "dirty": True, "branch": "release/0.8.25"},
    )
    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="clean"):
        scale_02_slice40.require_clean_execution(
            {"execution": execution, "candidate": {"source_commit": "a" * 40}},
            require_candidate_product_tree=False,
        )


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

    one_operation = {
        **config()["workload"],  # type: ignore[dict-item]
        "records": 1,
        "batch_size": 1,
        "expected_receipts": 1,
    }
    result.update(
        records=1,
        batch_size=1,
        receipt_count=1,
        operation_count=1,
        pending_receipt_count=1,
        pending_cursor_count=1,
    )
    scale_02_slice40.validate_worker_counts(result, one_operation)


def status_result(*, device: str, generation_p95: float = 0.02) -> dict[str, object]:
    return {
        "schema_version": "scale-02-slice40-status.v2",
        "build_variant": device,
        "status_compute_device": "cpu",
        "measurement_embedder": "fixed-test-cpu",
        "records": 50_000,
        "warmups": 100,
        "samples": 1_000,
        "errors": 0,
        "timeouts": 0,
        "cold_generation_ms": 75.0,
        "cold_mutation_ms": 75.0,
        "configuration_transition_ms": 1.0,
        "post_transition_generation_ms": 80.0,
        "post_transition_mutation_ms": 0.1,
        "generation_p95_ms": generation_p95,
        "generation_p99_ms": 0.03,
        "mutation_p95_ms": 0.02,
        "mutation_p99_ms": 0.03,
        "steady_full_owner_scans": 0,
        "epoch_rollover_full_owner_scans": 1,
        "query_plans": ["unregistered_node: SEARCH r USING INDEX example"],
    }


def test_status_verdict_requires_five_exact_cpu_cuda_repetitions() -> None:
    cpu = [status_result(device="cpu") for _ in range(5)]
    cuda = [status_result(device="cuda", generation_p95=0.03) for _ in range(5)]
    assert (
        scale_02_slice40.status_verdict(
            cpu,
            cuda,
            config()["workload"],
            config()["policy"],  # type: ignore[arg-type]
        )
        == "pass"
    )

    cuda[4]["generation_p95_ms"] = 2.03
    assert (
        scale_02_slice40.status_verdict(
            cpu,
            cuda,
            config()["workload"],
            config()["policy"],  # type: ignore[arg-type]
        )
        == "fail"
    )

    with pytest.raises(scale_02_slice40.Slice40ScaleError, match="repetitions"):
        scale_02_slice40.status_verdict(
            cpu[:4],
            cuda,
            config()["workload"],
            config()["policy"],  # type: ignore[arg-type]
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


def test_status_database_observation_is_exact_and_retained(tmp_path) -> None:
    path = tmp_path / "status.fathomdb"
    with sqlite3.connect(path) as connection:
        connection.executescript(
            """
            CREATE TABLE _fathomdb_projection_generations(
              generation_id TEXT PRIMARY KEY,
              role TEXT NOT NULL
            );
            INSERT INTO _fathomdb_projection_generations VALUES('pgen1:00', 'retired');
            INSERT INTO _fathomdb_projection_generations VALUES('pgen1:01', 'serving');
            """
        )

    observation = scale_02_slice40.inspect_status_database(path)
    assert observation == {
        "database_bytes": path.stat().st_size,
        "generation_history_rows": 2,
        "page_count": observation["page_count"],
        "page_size_bytes": observation["page_size_bytes"],
        "serving_generation_rows": 1,
        "wal_bytes": 0,
    }
    assert observation["page_count"] > 0
    assert observation["page_size_bytes"] > 0
