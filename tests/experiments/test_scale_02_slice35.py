from __future__ import annotations

import copy

import pytest

from experiments import scale_02_slice35


def config() -> dict[str, object]:
    return {
        "schema_version": "scale-02-slice35.v1",
        "program_track": "SCALE-02",
        "release": "0.8.25",
        "approval": {
            "state": "approved",
            "approved_by": "HITL",
            "approved_at": "2026-09-04",
        },
        "baseline": {
            "source_commit": "a" * 40,
            "wheel": "data/baseline.whl",
            "wheel_sha256": "b" * 64,
            "python_extension_sha256": "c" * 64,
            "fathomdb_bin": "data/baseline-bin",
            "fathomdb_bin_sha256": "d" * 64,
        },
        "candidate": {
            "source_commit": "e" * 40,
            "wheel": "data/candidate.whl",
            "wheel_sha256": "f" * 64,
            "python_extension_sha256": "1" * 64,
            "fathomdb_bin": "data/candidate-bin",
            "fathomdb_bin_sha256": "2" * 64,
        },
        "inputs": {
            "base_config": "experiments/configs/scale-02/a0-envelope.v2.json",
            "base_config_sha256": "3" * 64,
        },
        "workload": {
            "records": 10_000,
            "repetitions": 5,
            "warmups": 100,
            "steady_queries": 1_000,
            "query_order_seed": "0x5CA1E025350001",
            "bootstrap_seed": "0x5CA1E02535B007",
            "bootstrap_resamples": 2_000,
        },
        "policy": {
            "confidence": 0.95,
            "max_relative_regression": 0.03,
            "metrics": ["p50", "p95"],
        },
        "artifact_root": "data/performance-benchmarking/scale-02/slice35-runs",
        "claim_boundary": "legacy_search_non_regression_only",
    }


def test_config_rejects_unknown_fields() -> None:
    document = config()
    document["surprise"] = True
    with pytest.raises(scale_02_slice35.Slice35ScaleError, match="unknown"):
        scale_02_slice35.resolve_config(document, validate_files=False)


def test_config_pins_the_registered_workload() -> None:
    document = config()
    changed = copy.deepcopy(document)
    changed["workload"]["repetitions"] = 4  # type: ignore[index]
    with pytest.raises(scale_02_slice35.Slice35ScaleError, match="workload"):
        scale_02_slice35.resolve_config(changed, validate_files=False)


def test_v3_requires_a_closed_measurement_plan_reference() -> None:
    document = config()
    document["schema_version"] = "scale-02-slice35.v3"
    document["measurement_plan"] = {
        "path": "experiments/configs/scale-02/slice35-final.measurement-plan.v2.json",
        "sha256": "4" * 64,
        "plan_id": "slice35-final",
    }

    resolved = scale_02_slice35.resolve_config(document, validate_files=False)
    assert resolved["measurement_plan"] == document["measurement_plan"]

    document.pop("measurement_plan")
    with pytest.raises(scale_02_slice35.Slice35ScaleError, match="measurement_plan"):
        scale_02_slice35.resolve_config(document, validate_files=False)


def test_v3_classification_is_complete_and_exactly_counts_search_calls() -> None:
    authority = {
        "plan_id": "slice35-final",
        "components": [{"id": "component"}],
        "call_paths": [{"id": "path"}],
        "comparisons": [{"id": "comparison"}],
        "metric_bindings": [{"id": "metric"}],
        "metric_exclusions": [{"json_pointer": "/control"}],
        "claims": [{"id": "claim"}],
    }
    artifacts = [
        {"id": "metrics"},
        {"id": "runner"},
        {"id": "invocation"},
    ]

    document = scale_02_slice35.classification_document(
        run_id="run-id",
        authority=authority,
        source_artifacts=artifacts,
        measurement_plan_sha256="4" * 64,
        search_call_counts={"baseline": 5_500, "candidate": 5_500},
    )

    assert document["outcome"] == "complete"
    assert document["source_artifacts"] == artifacts
    assert [item["call_count"] for item in document["execution_witnesses"]] == [
        5_500,
        5_500,
    ]
    assert document["execution_witnesses"][0]["source_artifact_ids"] == [
        "metrics",
        "runner",
        "invocation",
    ]
    assert document["classification_id"]


def test_paired_bootstrap_upper_and_verdict_are_deterministic() -> None:
    baseline = [10.0, 10.1, 9.9, 10.2, 9.8]
    candidate = [10.1, 10.2, 10.0, 10.3, 9.9]
    first = scale_02_slice35.paired_relative_upper(
        baseline, candidate, seed=17, resamples=2_000
    )
    second = scale_02_slice35.paired_relative_upper(
        baseline, candidate, seed=17, resamples=2_000
    )
    assert first == second
    assert 0.0 < first < 0.03
    assert scale_02_slice35.regression_verdict({"p50": first, "p95": first}, 0.03) == "pass"
    assert scale_02_slice35.regression_verdict({"p50": 0.031, "p95": first}, 0.03) == "fail"
