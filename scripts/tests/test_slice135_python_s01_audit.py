"""Independent checks for retained installed-Python S01 pilot blocks."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path

import pytest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_python_s01_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_python_s01_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
EVIDENCE = (
    Path(__file__).resolve().parents[2]
    / "dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s01-noise-pilot"
)


def test_audit_recomputes_retained_valid_blocks() -> None:
    report = MODULE.audit_collection(EVIDENCE)
    assert report["valid_blocks"] == 10
    assert report["invalid_retained"] == ["invalid-venv-symlink"]
    assert report["sizes"]["32"]["blocks"] == 5
    assert report["sizes"]["256"]["blocks"] == 5
    assert report["sizes"]["32"]["total_checked_attempts"] == 1530
    assert report["sizes"]["256"]["total_checked_attempts"] == 1530
    assert report["sizes"]["32"]["cells"]["vector"]["p99"] == "unsupported"


def test_audit_rejects_semantic_mutant_despite_claimed_success() -> None:
    raw = json.loads((EVIDENCE / "32-01/raw.json").read_text())
    raw = copy.deepcopy(raw)
    raw["cells"]["text"]["warm"][0]["ids"] = ["B"]
    with pytest.raises(ValueError, match="text anchor"):
        MODULE.audit_observations(raw, 32)


def test_audit_rejects_bad_summary_even_with_valid_raw() -> None:
    raw = json.loads((EVIDENCE / "32-01/raw.json").read_text())
    summary = json.loads((EVIDENCE / "32-01/summary.json").read_text())
    summary["cells"]["hybrid"]["warm_p95_ns"] = 1
    with pytest.raises(ValueError, match="percentile"):
        MODULE.audit_observations(raw, 32, summary)
