"""Focused contracts for Performance Gauntlet v1.2 compatibility adapters."""

from __future__ import annotations

import hashlib
import json
import runpy
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
PROTECTED = ROOT / "scripts" / "perf-experiments" / "run-protected-production.py"
PROTECTED_MANIFEST = (
    ROOT / "experiments" / "configs" / "gauntlet" / "protected-production-v1.json"
)
CE = ROOT / "scripts" / "perf-experiments" / "run-ce-profile.py"


def test_protected_directional_manifest_is_separate_and_strict(tmp_path: Path) -> None:
    adapter = runpy.run_path(str(PROTECTED))
    runner = adapter["_load_runner"]()

    document = adapter["_load_manifest"](PROTECTED_MANIFEST, runner)

    assert document["fixtures"] == ["scale02", "ac013"]
    assert document["treatment"] == "production"
    assert document["repetitions_per_fixture"] == 3
    assert document["environment_policy"]["max_swap_io_delta"] == 1024

    tampered = json.loads(PROTECTED_MANIFEST.read_text(encoding="utf-8"))
    tampered["repetitions_per_fixture"] = 2
    path = tmp_path / "tampered.json"
    path.write_text(json.dumps(tampered), encoding="utf-8")
    with pytest.raises(ValueError, match="workload identity"):
        adapter["_load_manifest"](path, runner)


def test_protected_receipt_requires_six_exact_valid_cells() -> None:
    adapter = runpy.run_path(str(PROTECTED))
    cells = [
        {
            "fixture": fixture,
            "treatment": "production",
            "ordinal": ordinal,
            "environment_valid": True,
        }
        for fixture in ("scale02", "ac013")
        for ordinal in range(1, 4)
    ]

    adapter["_validate_receipt"]({"cells": cells})
    with pytest.raises(ValueError, match="six valid cells"):
        adapter["_validate_receipt"]({"cells": cells[:-1]})


def test_ce_adapter_pins_frozen_inputs_and_classifies_only_ratio_failure() -> None:
    adapter = runpy.run_path(str(CE))
    manifest = ROOT / "dev" / "plans" / "0.8.25" / "features" / "slice-72" / "ce-profile-manifest.json"
    cpu = ROOT / "dev" / "plans" / "runs" / "0.8.25-slice-72" / "baseline-cpu" / "baseline-cpu.json"
    cuda = ROOT / "dev" / "plans" / "runs" / "0.8.25-slice-72" / "baseline-cuda" / "baseline-cuda.json"

    assert hashlib.sha256(manifest.read_bytes()).hexdigest() == adapter["MANIFEST_SHA256"]
    assert hashlib.sha256(cpu.read_bytes()).hexdigest() == adapter["BASELINE_SHA256"]["cpu"]
    assert hashlib.sha256(cuda.read_bytes()).hexdigest() == adapter["BASELINE_SHA256"]["cuda"]
    assert (
        adapter["ratio_only_failure"](
            "FAIL Slice 72 CE profile: cuda engine p95 regression 1.200000 exceeds 1.100000\n"
        )
        == "cuda engine p95 regression 1.200000 exceeds 1.100000"
    )
    assert adapter["ratio_only_failure"](
        "FAIL Slice 72 CE profile: candidate artifact digest mismatch\n"
    ) is None
