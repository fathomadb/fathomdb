from __future__ import annotations

import copy
import json
import runpy
from pathlib import Path

import pytest

from experiments.release_0825_slice71b import (
    Slice71BContractError,
    attribution_classification,
    classify_recovery,
    validate_attribution_manifest,
    validate_attribution_receipt,
)


ROOT = Path(__file__).resolve().parents[2]
CONFIG_ROOT = ROOT / "experiments" / "configs"
MANIFEST_PATH = CONFIG_ROOT / "release-0825-slice71b-attribution-manifest.v1.json"
RUNNER = runpy.run_path(str(ROOT / "scripts" / "perf-experiments" / "run_slice71b.py"))


def manifest() -> dict[str, object]:
    return json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))


def valid_cell(
    fixture: str,
    treatment: str,
    ordinal: int,
    *,
    ack_ms: float = 10.0,
    total_ms: float = 12.0,
) -> dict[str, object]:
    visibility = {
        "production": (1, 30_001, 30_001, "a" * 64, "b" * 64, "b" * 64),
        "generation_only": (1, 30_001, 30_001, "a" * 64, "a" * 64, "a" * 64),
        "no_op": (1, 1, 1, "a" * 64, "a" * 64, "a" * 64),
    }[treatment]
    return {
        "fixture": fixture,
        "treatment": treatment,
        "ordinal": ordinal,
        "source_ref": "1" * 40,
        "source_tree_dirty": False,
        "probe_sha256": "2" * 64,
        "probe_lock_sha256": "6" * 64,
        "runner_sha256": "7" * 64,
        "executable_sha256": "8" * 64,
        "executable_path": "/tmp/slice71b-probe",
        "fixture_sha256": "3" * 64,
        "started_at": "2026-09-08T00:00:00Z",
        "finished_at": "2026-09-08T00:00:01Z",
        "environment_valid": True,
        "environment_sha256": "9" * 64,
        "environment_path": "dev/plans/runs/0.8.25-slice-71/71b/environment.json",
        "build_identity": {
            "cargo": "cargo 1.0.0",
            "rustc": "rustc 1.0.0",
            "rustc_verbose_sha256": "a" * 64,
            "profile": "release",
        },
        "runtime_identity": {
            "sqlite_version": "3.50.0",
            "sqlite_source_id": "source-id",
            "libsqlite3_sys": "0.38.0",
        },
        "raw_log_sha256": "4" * 64,
        "raw_log_path": "dev/plans/runs/0.8.25-slice-71/71b/raw.log",
        "metrics": {
            "records": 10_000,
            "batch_size": 256 if fixture == "scale02" else 1_024,
            "transactions": 40 if fixture == "scale02" else 10,
            "ingest_ack_ms": ack_ms,
            "projection_drain_ms": total_ms - ack_ms,
            "total_ms": total_ms,
            "generation_before": visibility[0],
            "generation_after_ack": visibility[1],
            "generation_after_drain": visibility[2],
            "nonce_before": visibility[3],
            "nonce_after_ack": visibility[4],
            "nonce_after_drain": visibility[5],
            "trigger_inventory": 54,
            "database_bytes": 1,
            "wal_bytes": 1,
            "process_cpu_seconds": 0.01,
            "peak_rss_bytes": 1,
        },
    }


def valid_attribution_receipt() -> dict[str, object]:
    order = manifest()["attribution_order"]
    cells = []
    for fixture in ("scale02", "ac013"):
        ordinals = {"production": 0, "generation_only": 0, "no_op": 0}
        for treatment in order:  # type: ignore[union-attr]
            ordinals[treatment] += 1
            timing = {
                "production": (20.0, 24.0),
                "generation_only": (16.0, 20.0),
                "no_op": (10.0, 12.0),
            }[treatment]
            cell = valid_cell(
                fixture,
                treatment,
                ordinals[treatment],
                ack_ms=timing[0],
                total_ms=timing[1],
            )
            position = len(cells)
            cell["started_at"] = f"2026-09-08T00:{position:02d}:00Z"
            cell["finished_at"] = f"2026-09-08T00:{position:02d}:30Z"
            cells.append(cell)
    return {
        "schema_version": "slice71b-attribution-receipt.v1",
        "manifest_sha256": "5" * 64,
        "started_at": "2026-09-08T00:00:00Z",
        "finished_at": "2026-09-08T00:18:00Z",
        "cells": cells,
        "classification": {
            "state": "supported",
            "supported_causes": ["per_row_visibility_update", "per_fire_nonce"],
            "conditional_preparation_factorial_required": False,
        },
        "failure": None,
        "errors": [],
    }


def test_checked_in_attribution_manifest_is_strict_and_valid() -> None:
    validate_attribution_manifest(manifest())


@pytest.mark.parametrize(
    ("path", "value"),
    [
        (("unexpected",), True),
        (("attribution_order",), ["production"] * 9),
        (("fixtures", "scale02", "batch_size_10k"), 1_024),
        (("fixtures", "ac013", "vector_dim"), 768),
        (("policy", "max_within_arm_spread_percent"), 10),
        (("treatments",), ["production", "no_op"]),
    ],
)
def test_attribution_manifest_rejects_drift(
    path: tuple[str, ...], value: object
) -> None:
    document = copy.deepcopy(manifest())
    target = document
    for key in path[:-1]:
        target = target[key]  # type: ignore[index,assignment]
    target[path[-1]] = value  # type: ignore[index]
    with pytest.raises(Slice71BContractError):
        validate_attribution_manifest(document)


def test_attribution_receipt_accepts_exact_complete_matrix() -> None:
    validate_attribution_receipt(
        valid_attribution_receipt(), manifest(), verify_hashes=False
    )


@pytest.mark.parametrize(
    ("path", "value"),
    [
        (("unexpected",), True),
        (("cells", 0, "metrics", "total_ms"), 0.0),
        (("cells", 0, "metrics", "transactions"), 39),
        (("cells", 0, "source_tree_dirty"), True),
        (("cells", 0, "environment_valid"), False),
        (("cells", 0, "environment_sha256"), ""),
        (("cells", 0, "runtime_identity", "sqlite_version"), ""),
        (("classification", "conditional_preparation_factorial_required"), True),
        (("classification", "supported_causes"), ["statement_preparation"]),
    ],
)
def test_attribution_receipt_rejects_drift(
    path: tuple[object, ...], value: object
) -> None:
    document = valid_attribution_receipt()
    target: object = document
    for key in path[:-1]:
        target = target[key]  # type: ignore[index]
    target[path[-1]] = value  # type: ignore[index]
    with pytest.raises(Slice71BContractError):
        validate_attribution_receipt(document, manifest(), verify_hashes=False)


def test_attribution_receipt_rejects_ack_total_inconsistency() -> None:
    document = valid_attribution_receipt()
    document["cells"][0]["metrics"]["projection_drain_ms"] = 9.0  # type: ignore[index]
    with pytest.raises(Slice71BContractError, match="total_ms"):
        validate_attribution_receipt(document, manifest(), verify_hashes=False)


def test_attribution_receipt_accepts_canonical_partial_abort() -> None:
    document = valid_attribution_receipt()
    document["cells"] = document["cells"][:4]
    document["finished_at"] = "2026-09-08T00:04:00Z"
    document["classification"] = {
        "state": "probe_failed",
        "supported_causes": [],
        "conditional_preparation_factorial_required": False,
    }
    document["failure"] = {
        "state": "probe_failed",
        "fixture": "scale02",
        "treatment": "generation_only",
        "ordinal": 2,
        "occurred_at": "2026-09-08T00:04:00Z",
        "artifacts": [
            {
                "kind": "attempt_disposition",
                "path": "dev/plans/runs/0.8.25-slice-71/71b/attempt.json",
                "sha256": "b" * 64,
            },
            {
                "kind": "environment",
                "path": "dev/plans/runs/0.8.25-slice-71/71b/environment.json",
                "sha256": "c" * 64,
            },
            {
                "kind": "raw_log",
                "path": "dev/plans/runs/0.8.25-slice-71/71b/raw.log",
                "sha256": "d" * 64,
            },
        ],
    }
    document["errors"] = ["scale02/no_op-2: retained probe failure"]
    validate_attribution_receipt(document, manifest(), verify_hashes=False)


def test_attribution_receipt_rejects_unexplained_partial_matrix() -> None:
    document = valid_attribution_receipt()
    document["cells"] = document["cells"][:4]
    with pytest.raises(Slice71BContractError, match="exact 18-cell"):
        validate_attribution_receipt(document, manifest(), verify_hashes=False)


def test_attribution_receipt_rejects_unbound_partial_abort() -> None:
    document = valid_attribution_receipt()
    document["cells"] = document["cells"][:4]
    document["finished_at"] = "2026-09-08T00:04:00Z"
    document["classification"] = {
        "state": "probe_failed",
        "supported_causes": [],
        "conditional_preparation_factorial_required": False,
    }
    document["errors"] = ["unbound failure"]
    document["failure"] = None
    with pytest.raises(Slice71BContractError, match="failure"):
        validate_attribution_receipt(document, manifest(), verify_hashes=False)


def test_production_signature_rejects_nonce_change_without_generation_change() -> None:
    document = valid_attribution_receipt()
    document["cells"][0]["metrics"]["nonce_after_drain"] = "c" * 64
    with pytest.raises(Slice71BContractError, match="production"):
        validate_attribution_receipt(document, manifest(), verify_hashes=False)


def test_runner_exposes_only_the_exact_attribution_sequence() -> None:
    sequence = RUNNER["attribution_sequence"](manifest())
    expected = []
    for fixture in ("scale02", "ac013"):
        ordinals = {"production": 0, "generation_only": 0, "no_op": 0}
        for treatment in manifest()["attribution_order"]:
            ordinals[treatment] += 1
            expected.append((fixture, treatment, ordinals[treatment]))
    assert sequence == expected


def test_runner_binds_failed_attempt_artifacts(tmp_path: Path) -> None:
    environment = tmp_path / "environment.json"
    raw_log = tmp_path / "raw.log"
    environment.write_text("environment\n", encoding="utf-8")
    raw_log.write_text("probe output\n", encoding="utf-8")
    failure = RUNNER["_failure_disposition"](
        state="probe_failed",
        message="failed",
        root=tmp_path,
        fixture="scale02",
        treatment="production",
        ordinal=1,
    )
    assert {artifact["kind"] for artifact in failure["artifacts"]} == {
        "environment",
        "raw_log",
        "attempt_disposition",
    }
    for artifact in failure["artifacts"]:
        assert len(artifact["sha256"]) == 64
    assert (tmp_path / "attempt.json").is_file()


def test_runner_distinguishes_build_and_post_build_harness_failures() -> None:
    assert RUNNER["_unexpected_failure_state"](False) == "build_failed"
    assert RUNNER["_unexpected_failure_state"](True) == "harness_failed"


def test_runner_production_signature_requires_atomic_drain_change() -> None:
    metrics = valid_cell("scale02", "production", 1)["metrics"]
    metrics["nonce_after_drain"] = "c" * 64
    assert not RUNNER["_treatment_signature_valid"]("production", metrics)


def test_attribution_classifier_requires_measured_materiality() -> None:
    cells = valid_attribution_receipt()["cells"]
    assert attribution_classification(cells) == {
        "state": "supported",
        "supported_causes": ["per_row_visibility_update", "per_fire_nonce"],
        "conditional_preparation_factorial_required": False,
    }


def test_attribution_classifier_seals_unresolved_factorial_extension() -> None:
    cells = valid_attribution_receipt()["cells"]
    for cell in cells:  # type: ignore[union-attr]
        cell["metrics"]["ingest_ack_ms"] = 10.0
        cell["metrics"]["projection_drain_ms"] = 2.0
        cell["metrics"]["total_ms"] = 12.0
    assert attribution_classification(cells) == {
        "state": "unresolved",
        "supported_causes": [],
        "conditional_preparation_factorial_required": True,
    }


def test_recovery_classifier_requires_both_10k_boundaries() -> None:
    medians = {
        "scale02": {
            10_000: {
                "baseline": {"ingest_ack_ms": 100.0, "total_ms": 100.0},
                "fixed": {"ingest_ack_ms": 119.9, "total_ms": 119.9},
            }
        },
        "ac013": {
            10_000: {
                "baseline": {"ingest_ack_ms": 100.0, "total_ms": 100.0},
                "fixed": {"ingest_ack_ms": 80.0, "total_ms": 120.1},
            }
        },
    }
    assert classify_recovery(medians) == "missed_10k_boundary"


@pytest.mark.parametrize(
    ("fixed", "expected"),
    [
        (1.24, "pass"),
        (1.26, "small_write_regression"),
        (1.11, "pass"),
    ],
)
def test_recovery_classifier_uses_joint_small_write_threshold(
    fixed: float, expected: str
) -> None:
    medians: dict[str, dict[int, dict[str, dict[str, float]]]] = {}
    for fixture in ("scale02", "ac013"):
        medians[fixture] = {
            1: {
                "baseline": {"ingest_ack_ms": 1.0, "total_ms": 1.0},
                "fixed": {"ingest_ack_ms": fixed, "total_ms": fixed},
            },
            10_000: {
                "baseline": {"ingest_ack_ms": 100.0, "total_ms": 100.0},
                "fixed": {"ingest_ack_ms": 100.0, "total_ms": 100.0},
            },
        }
    assert classify_recovery(medians) == expected
