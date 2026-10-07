"""Frozen-order and independent paired-statistic checks for Slice 135 E01–E12."""

from __future__ import annotations

import copy
from pathlib import Path
import sys

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from slice135_e12_pair_audit import paired_statistics, validate_schedule  # noqa: E402


def fixture() -> tuple[dict, dict, dict]:
    frozen = {
        "workload": {
            "pair_order": [["baseline", "candidate"]],
            "minimum_idle_seconds_between_blocks": 20,
            "query_cells": ["text"],
            "lifecycle_cells": ["close"],
            "query_samples_per_block": 1000,
            "lifecycle_samples_per_block": 100,
        }
    }
    blocks = []
    for group, first in (("query", 100), ("lifecycle", 145)):
        for position, version in enumerate(("baseline", "candidate"), start=1):
            start = first + (position - 1) * 22
            blocks.append({"group": group, "pair": 1, "position": position,
                           "version": version, "name": f"{group}-pair-1-{position}-{version}",
                           "start_unix": start, "end_unix": start + 2, "status": "audited"})
    schedule = {"blocks": blocks, "minimum_idle_seconds": 20}
    summaries = {
        "query-pair-1-1-baseline": {"text": {"valid_samples": 1000, "p50_ns": 100, "p95_ns": 200, "p99_ns": 300}},
        "query-pair-1-2-candidate": {"text": {"valid_samples": 1000, "p50_ns": 110, "p95_ns": 180, "p99_ns": 330}},
        "lifecycle-pair-1-1-baseline": {"close": {"valid_samples": 100, "p50_ns": 400, "p95_ns": 500}},
        "lifecycle-pair-1-2-candidate": {"close": {"valid_samples": 100, "p50_ns": 360, "p95_ns": 550}},
    }
    return frozen, schedule, summaries


def test_frozen_schedule_and_paired_deltas() -> None:
    frozen, schedule, summaries = fixture()
    ordered = validate_schedule(schedule, frozen)
    result = paired_statistics(ordered, summaries, frozen)
    assert result["text"]["p50"]["pair_deltas_pct"] == [10.0]
    assert result["text"]["p95"]["pair_deltas_pct"] == [-10.0]
    assert result["text"]["p99"]["pair_deltas_pct"] == [10.0]
    assert result["close"]["p50"]["pair_deltas_pct"] == [-10.0]
    assert "p99" not in result["close"]


@pytest.mark.parametrize("change", ["order", "gap", "status", "missing", "sample"])
def test_schedule_or_sample_mutation_is_rejected(change: str) -> None:
    frozen, schedule, summaries = fixture()
    schedule, summaries = copy.deepcopy(schedule), copy.deepcopy(summaries)
    if change == "order":
        schedule["blocks"][1]["version"] = "baseline"
    elif change == "gap":
        schedule["blocks"][1]["start_unix"] = 120
    elif change == "status":
        schedule["blocks"][2]["status"] = "invalid"
    elif change == "missing":
        schedule["blocks"].pop()
    else:
        summaries["query-pair-1-2-candidate"]["text"]["valid_samples"] = 999
    with pytest.raises(ValueError):
        ordered = validate_schedule(schedule, frozen)
        paired_statistics(ordered, summaries, frozen)
