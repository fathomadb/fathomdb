"""Negative controls for independent S01 Python paired-receipt analysis."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/slice135_python_s01_pair_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_python_s01_pair_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
EVIDENCE = (
    ROOT / "dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s01-paired"
)
PROTOCOL = (
    ROOT / "dev/plans/0.8.27/features/slice-135/s01-python-comparison-protocol.json"
)


def test_pair_audit_recomputes_all_retained_blocks() -> None:
    report = MODULE.audit_collection(EVIDENCE, PROTOCOL)
    assert report["valid_blocks"] == 20
    assert report["checked_attempts"] == 60120
    assert report["excluded_attempts"] == ["invalid-pre-p99-fix"]
    for size in ("32", "256"):
        assert report["sizes"][size]["pairs"] == 5
        assert (
            len(report["sizes"][size]["cells"]["vector"]["p99"]["pair_deltas_pct"]) == 5
        )


def test_pair_audit_rejects_wrong_text_id_even_when_worker_claims_success() -> None:
    raw = json.loads((EVIDENCE / "32-01-baseline/raw.json").read_text())
    changed = copy.deepcopy(raw)
    changed["cells"]["text"]["warm"][0]["ids"] = ["B"]
    with pytest.raises(ValueError, match="text anchor"):
        MODULE.audit_observations(changed, size=32, samples=1000)


def test_pair_audit_rejects_wrong_p99_summary() -> None:
    raw = json.loads((EVIDENCE / "32-01-baseline/raw.json").read_text())
    summary = json.loads((EVIDENCE / "32-01-baseline/summary.json").read_text())
    summary["cells"]["hybrid"]["warm_p99_ns"] = 1
    with pytest.raises(ValueError, match="p99"):
        MODULE.audit_observations(raw, size=32, samples=1000, summary=summary)


def test_child_major_faults_are_reported_as_warning_without_losing_samples() -> None:
    resources = {
        "method": "gnu-time",
        "unsupported": [],
        "swap_events": 0,
        "major_faults": 20,
    }
    assert MODULE.child_resource_warnings(resources) == [
        "child major faults: 20"
    ]
    resources["swap_events"] = 1
    with pytest.raises(ValueError, match="resource invalidator"):
        MODULE.child_resource_warnings(resources)


def test_warning_block_names_keep_host_swap_separate_from_major_faults() -> None:
    blocks = [
        {"directory": "a", "warnings": ["child major faults: 20"]},
        {"directory": "b", "warnings": ["host swap drift: 2 pages; child swap events: 0"]},
    ]
    assert MODULE.warning_block_names(blocks, "host swap drift:") == ["b"]
    assert MODULE.warning_block_names(blocks, "child major faults:") == ["a"]
