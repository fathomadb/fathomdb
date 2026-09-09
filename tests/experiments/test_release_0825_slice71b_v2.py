from __future__ import annotations

import copy
import json
import runpy
from pathlib import Path

import pytest

from experiments.release_0825_slice71b import (
    Slice71BContractError,
    validate_attribution_manifest as validate_v1_manifest,
    validate_attribution_receipt as validate_v1_receipt,
)
from experiments.release_0825_slice71b_v2 import (
    validate_attribution_manifest,
    validate_attribution_receipt,
)


ROOT = Path(__file__).resolve().parents[2]
CONFIG_ROOT = ROOT / "experiments" / "configs"
V1_MANIFEST_PATH = CONFIG_ROOT / "release-0825-slice71b-attribution-manifest.v1.json"
V2_MANIFEST_PATH = CONFIG_ROOT / "release-0825-slice71b-attribution-manifest.v2.json"
V2_RUNNER_PATH = ROOT / "scripts" / "perf-experiments" / "run_slice71b_v2.py"


def manifest_v1() -> dict[str, object]:
    return json.loads(V1_MANIFEST_PATH.read_text(encoding="utf-8"))


def manifest_v2() -> dict[str, object]:
    return json.loads(V2_MANIFEST_PATH.read_text(encoding="utf-8"))


def valid_cell(
    fixture: str,
    treatment: str,
    ordinal: int,
    *,
    ack_ms: float,
    total_ms: float,
) -> dict[str, object]:
    generation_before = 1 if fixture == "scale02" else 5
    terminal_delta = 30_000 if fixture == "scale02" else 50_000
    terminal = generation_before + terminal_delta
    if treatment == "production":
        generations = (generation_before, terminal, terminal)
        nonces = ("a" * 64, "b" * 64, "b" * 64)
    elif treatment == "generation_only":
        generations = (generation_before, terminal, terminal)
        nonces = ("a" * 64, "a" * 64, "a" * 64)
    else:
        generations = (generation_before, generation_before, generation_before)
        nonces = ("a" * 64, "a" * 64, "a" * 64)
    return {
        "fixture": fixture,
        "treatment": treatment,
        "ordinal": ordinal,
        "source_ref": "1" * 40,
        "source_tree_dirty": False,
        "probe_sha256": "2" * 64,
        "probe_lock_sha256": "3" * 64,
        "runner_sha256": "4" * 64,
        "executable_sha256": "5" * 64,
        "executable_path": "/tmp/slice71b-v2-probe",
        "fixture_sha256": "6" * 64,
        "started_at": "2026-09-09T00:00:00Z",
        "finished_at": "2026-09-09T00:00:30Z",
        "environment_valid": True,
        "environment_sha256": "7" * 64,
        "environment_path": "dev/plans/runs/0.8.25-slice-71/71b/v2-env.json",
        "raw_log_sha256": "8" * 64,
        "raw_log_path": "dev/plans/runs/0.8.25-slice-71/71b/v2-raw.log",
        "build_identity": {
            "cargo": "cargo 1.0.0",
            "rustc": "rustc 1.0.0",
            "rustc_verbose_sha256": "9" * 64,
            "profile": "release",
        },
        "runtime_identity": {
            "sqlite_version": "3.50.0",
            "sqlite_source_id": "source-id",
            "libsqlite3_sys": "0.38.0",
        },
        "metrics": {
            "records": 10_000,
            "batch_size": 256 if fixture == "scale02" else 1_024,
            "transactions": 40 if fixture == "scale02" else 10,
            "ingest_ack_ms": ack_ms,
            "projection_drain_ms": total_ms - ack_ms,
            "total_ms": total_ms,
            "generation_before": generations[0],
            "generation_after_ack": generations[1],
            "generation_after_drain": generations[2],
            "nonce_before": nonces[0],
            "nonce_after_ack": nonces[1],
            "nonce_after_drain": nonces[2],
            "trigger_inventory": 54,
            "database_bytes": 1,
            "wal_bytes": 1,
            "process_cpu_seconds": 0.01,
            "peak_rss_bytes": 1,
        },
    }


def valid_receipt_v2() -> dict[str, object]:
    cells = []
    ordinals = {
        fixture: {treatment: 0 for treatment in ("production", "generation_only", "no_op")}
        for fixture in ("scale02", "ac013")
    }
    ac013_ack = {
        "production": [2.0, 8.0, 5.0],
        "generation_only": [2.0, 8.0, 5.0],
        "no_op": [5.0, 5.1, 4.9],
    }
    total = {"production": 24.0, "generation_only": 20.0, "no_op": 12.0}
    for fixture in ("scale02", "ac013"):
        for treatment in manifest_v2()["attribution_order"]:  # type: ignore[union-attr]
            ordinals[fixture][treatment] += 1
            ordinal = ordinals[fixture][treatment]
            ack = total[treatment] - 2.0
            if fixture == "ac013":
                ack = ac013_ack[treatment][ordinal - 1]
            cell = valid_cell(
                fixture,
                treatment,
                ordinal,
                ack_ms=ack,
                total_ms=total[treatment],
            )
            position = len(cells)
            cell["started_at"] = f"2026-09-09T00:{position:02d}:00Z"
            cell["finished_at"] = f"2026-09-09T00:{position:02d}:30Z"
            cells.append(cell)
    return {
        "schema_version": "slice71b-attribution-receipt.v2",
        "manifest_sha256": "a" * 64,
        "started_at": "2026-09-09T00:00:00Z",
        "finished_at": "2026-09-09T00:18:00Z",
        "cells": cells,
        "classification": {
            "state": "supported",
            "supported_causes": ["per_row_visibility_update", "per_fire_nonce"],
            "conditional_preparation_factorial_required": False,
        },
        "failure": None,
        "errors": [],
    }


def test_v1_contract_remains_valid_and_rejects_v2() -> None:
    v1 = manifest_v1()
    validate_v1_manifest(v1)
    with pytest.raises(Slice71BContractError, match="schema_version"):
        validate_v1_receipt(valid_receipt_v2(), v1, verify_hashes=False)


def test_checked_in_v2_manifest_and_schemas_are_valid() -> None:
    validate_attribution_manifest(manifest_v2())
    for name in (
        "release-0825-slice71b-attribution-manifest.v2.schema.json",
        "release-0825-slice71b-attribution-receipt.v2.schema.json",
    ):
        json.loads((CONFIG_ROOT / name).read_text(encoding="utf-8"))


@pytest.mark.parametrize(
    ("path", "value"),
    [
        (("predecessor", "sha256"), "0" * 64),
        (("predecessor", "state"), "supported"),
        (("predecessor", "superseded_rule"), "total_ms"),
        (("policy", "ac013_spread_metrics"), ["ingest_ack_ms", "total_ms"]),
        (("policy", "ac013_terminal_generation_delta"), 49_999),
        (("raw_root",), "dev/plans/runs/0.8.25-slice-71/71b"),
    ],
)
def test_v2_manifest_rejects_protocol_drift(
    path: tuple[str, ...], value: object
) -> None:
    document = copy.deepcopy(manifest_v2())
    target = document
    for key in path[:-1]:
        target = target[key]  # type: ignore[index,assignment]
    target[path[-1]] = value  # type: ignore[index]
    with pytest.raises(Slice71BContractError):
        validate_attribution_manifest(document, verify_files=False)


def test_v2_accepts_ac013_ack_redistribution_with_stable_total_and_equal_work() -> None:
    validate_attribution_receipt(
        valid_receipt_v2(), manifest_v2(), verify_hashes=False
    )


def test_v2_rejects_scale02_ack_spread() -> None:
    receipt = valid_receipt_v2()
    scale_production = [
        cell
        for cell in receipt["cells"]  # type: ignore[union-attr]
        if cell["fixture"] == "scale02" and cell["treatment"] == "production"
    ]
    scale_production[0]["metrics"]["ingest_ack_ms"] = 2.0
    scale_production[0]["metrics"]["projection_drain_ms"] = 22.0
    with pytest.raises(Slice71BContractError, match="ingest_ack_ms spread"):
        validate_attribution_receipt(receipt, manifest_v2(), verify_hashes=False)


def test_v2_rejects_ac013_total_spread() -> None:
    receipt = valid_receipt_v2()
    ac013_generation = [
        cell
        for cell in receipt["cells"]  # type: ignore[union-attr]
        if cell["fixture"] == "ac013"
        and cell["treatment"] == "generation_only"
    ]
    ac013_generation[0]["metrics"]["total_ms"] = 30.0
    ac013_generation[0]["metrics"]["projection_drain_ms"] = 28.0
    with pytest.raises(Slice71BContractError, match="total_ms spread"):
        validate_attribution_receipt(receipt, manifest_v2(), verify_hashes=False)


def test_v2_rejects_unequal_ac013_terminal_work() -> None:
    receipt = valid_receipt_v2()
    ac013_generation = next(
        cell
        for cell in receipt["cells"]  # type: ignore[union-attr]
        if cell["fixture"] == "ac013"
        and cell["treatment"] == "generation_only"
    )
    ac013_generation["metrics"]["generation_after_drain"] = 50_004
    with pytest.raises(Slice71BContractError, match="terminal generation delta"):
        validate_attribution_receipt(receipt, manifest_v2(), verify_hashes=False)


def test_v2_rejects_ac013_no_op_work() -> None:
    receipt = valid_receipt_v2()
    ac013_no_op = next(
        cell
        for cell in receipt["cells"]  # type: ignore[union-attr]
        if cell["fixture"] == "ac013" and cell["treatment"] == "no_op"
    )
    ac013_no_op["metrics"]["generation_after_ack"] = 6
    ac013_no_op["metrics"]["generation_after_drain"] = 6
    with pytest.raises(Slice71BContractError, match="no-op"):
        validate_attribution_receipt(receipt, manifest_v2(), verify_hashes=False)


def test_v2_runner_uses_fixture_specific_spread_metrics() -> None:
    runner = runpy.run_path(str(V2_RUNNER_PATH))
    receipt = valid_receipt_v2()
    cells = receipt["cells"]  # type: ignore[assignment]
    assert runner["_arm_spread"](
        cells, manifest_v2(), "ac013", "generation_only"
    )[0] == "total_ms"
    assert runner["_arm_spread"](
        cells, manifest_v2(), "scale02", "production"
    )[0] == "ingest_ack_ms"
