"""Reject invalid S02 product results before accepting a pilot block."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_block as block  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
RECEIPT = (
    ROOT
    / "dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-baseline-noise-pilot"
)


def sample() -> tuple[dict, dict]:
    return (
        json.loads((RECEIPT / "block-01/sample-01.json").read_text()),
        json.loads((RECEIPT / "manifest.json").read_text()),
    )


def test_sample_guard_accepts_measured_state_and_rejects_retained_source() -> None:
    raw, manifest = sample()
    assert block.validate_sample(raw, manifest) == raw["whole_product_ns"]
    bad = deepcopy(raw)
    bad["observed"]["after_reopen"]["source_absent"] = False
    with pytest.raises(ValueError, match="reopened source"):
        block.validate_sample(bad, manifest)


def test_sample_guard_rejects_timer_or_artifact_mismatch() -> None:
    raw, manifest = sample()
    bad = deepcopy(raw)
    bad["whole_product_ns"] = 1
    with pytest.raises(ValueError, match="stage sum"):
        block.validate_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["artifact"]["wheel_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="wheel"):
        block.validate_sample(bad, manifest)
    bound = {
        **manifest,
        "corpus_sha256": raw["corpus_sha256"],
        "graph_records_sha256": raw["graph_records_sha256"],
    }
    bad = deepcopy(raw)
    bad["corpus_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="corpus"):
        block.validate_sample(bad, bound)


def test_even_sample_median_uses_declared_nearest_rank() -> None:
    assert block.nearest_rank_median([40, 10, 30, 20]) == 20


def test_frozen_pair_protocol_refuses_changed_artifact_or_sample_count() -> None:
    raw, manifest = sample()
    assert raw["source_sha"] == manifest["source_sha"]
    protocol = {
        "schema_version": 1,
        "status": "FROZEN_S02_PYTHON_PAIRED",
        "baseline": {
            "source_sha": manifest["source_sha"],
            "wheel_sha256": manifest["wheel_sha256"],
        },
        "candidate": {"source_sha": "1" * 40, "wheel_sha256": "2" * 64},
        "samples_per_block": 20,
        "block_runner_sha256": block.sha(Path(block.__file__)),
        "timed_runner_sha256": block.sha(block.RUNNER),
        "s01_helper_sha256": block.sha(block.S01_HELPER),
        "s02_helper_sha256": block.sha(block.S02_HELPER),
        "pilot_helper_sha256": block.sha(block.ROOT / "scripts/slice135_pilot.py"),
        "corpus_sha256": block.CORPUS_SHA256,
        "graph_records_sha256": block.GRAPH_RECORDS_SHA256,
    }
    block.validate_comparison_protocol(
        protocol,
        role="baseline",
        source_sha=manifest["source_sha"],
        wheel_sha256=manifest["wheel_sha256"],
        samples=20,
    )
    changed = deepcopy(protocol)
    changed["baseline"]["wheel_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="artifact"):
        block.validate_comparison_protocol(
            changed,
            role="baseline",
            source_sha=manifest["source_sha"],
            wheel_sha256=manifest["wheel_sha256"],
            samples=20,
        )
    with pytest.raises(ValueError, match="sample count"):
        block.validate_comparison_protocol(
            protocol,
            role="baseline",
            source_sha=manifest["source_sha"],
            wheel_sha256=manifest["wheel_sha256"],
            samples=3,
        )
    changed = deepcopy(protocol)
    changed["pilot_helper_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="pilot_helper_sha256"):
        block.validate_comparison_protocol(
            changed,
            role="baseline",
            source_sha=manifest["source_sha"],
            wheel_sha256=manifest["wheel_sha256"],
            samples=20,
        )


def test_committed_s02_subset_binds_both_roles_to_current_runner() -> None:
    path = (
        ROOT / "dev/plans/0.8.27/features/slice-135/s02-python-comparison-protocol.json"
    )
    protocol = json.loads(path.read_text())
    for role in ("baseline", "candidate"):
        block.validate_comparison_protocol(
            protocol,
            role=role,
            source_sha=protocol[role]["source_sha"],
            wheel_sha256=protocol[role]["wheel_sha256"],
            samples=20,
        )
