"""Ensure paired S02 recomputation rejects wrong product state and tails."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_pair_audit as audit  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
RAW = (
    ROOT
    / "dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-controlled-noise-pilot/block-01/sample-001.json"
)


def test_materialized_oracle_rejects_surviving_source_and_wrong_edge() -> None:
    raw = json.loads(RAW.read_text())
    audit.validate_materialized(raw)
    bad = deepcopy(raw)
    bad["observed"]["after_reopen"]["source_absent"] = False
    with pytest.raises(ValueError, match="source_absent"):
        audit.validate_materialized(bad)
    bad = deepcopy(raw)
    bad["observed"]["graph"]["edge_revision"] = "wrong"
    with pytest.raises(ValueError, match="graph evidence"):
        audit.validate_materialized(bad)


def test_nearest_rank_percentile_has_no_interpolation() -> None:
    assert audit.nearest_rank([40, 10, 30, 20], 0.5) == 20
    assert audit.nearest_rank([40, 10, 30, 20], 0.95) == 40
