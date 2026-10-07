"""Negative controls for the integrated installed-Python S01 pair audit."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s01_integrated_audit as audit  # noqa: E402


def test_order_audit_rejects_missing_and_wrong_role() -> None:
    expected = [(32, 1, "baseline"), (32, 1, "candidate")]
    records = [
        {
            "name": f"32-01-{role}",
            "rows": 32,
            "pair": 1,
            "role": role,
            "samples": 1000,
            "exit_code": 0,
            "attempt_status": "VALID_S01_PAIRED_BLOCK",
            "invalidators": [],
            "protocol_sha256": "a" * 64,
        }
        for _, _, role in expected
    ]
    audit.check_run_order(records, expected, "a" * 64)
    with pytest.raises(ValueError, match="count"):
        audit.check_run_order(records[:-1], expected, "a" * 64)
    changed = deepcopy(records)
    changed[1]["role"] = "baseline"
    with pytest.raises(ValueError, match="order"):
        audit.check_run_order(changed, expected, "a" * 64)


def test_pair_summary_recomputes_deltas_and_rejects_missing_role() -> None:
    blocks = [
        {
            "size": 32,
            "pair": pair,
            "role": role,
            "warnings": [],
            "cells": {
                "text": {
                    "warm_p50_ns": value,
                    "warm_p95_ns": value * 2,
                    "warm_p99_ns": value * 3,
                }
            },
        }
        for pair, values in ((1, (100, 110)), (2, (100, 120)))
        for role, value in zip(("baseline", "candidate"), values)
    ]
    summary = audit.summarize_pairs(blocks, size=32, pair_count=2, kinds=("text",))
    assert summary["text"]["p50"]["pair_deltas_pct"] == [10.0, 20.0]
    assert summary["text"]["p50"]["median_delta_pct"] == 15.0
    with pytest.raises(ValueError, match="missing"):
        audit.summarize_pairs(blocks[:-1], size=32, pair_count=2, kinds=("text",))
