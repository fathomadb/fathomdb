"""Independent paired contention audit rejects changed order and summaries."""

from __future__ import annotations

from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_contention_pair_audit as audit  # noqa: E402


def test_order_requires_every_alternating_block_and_idle() -> None:
    entries = [
        {
            "pair": pair,
            "role": role,
            "block": f"pair-{pair:02d}-{role}",
            "start_monotonic_ns": position * 20_000_000_000,
            "end_monotonic_ns": position * 20_000_000_000 + 5_000_000_000,
            "exit_code": 0,
        }
        for position, (pair, role) in enumerate(
            [(1, "baseline"), (1, "candidate"),
             (2, "candidate"), (2, "baseline")]
        )
    ]
    audit.validate_order(entries, pairs=2, minimum_idle_seconds=10)
    with pytest.raises(ValueError, match="order"):
        audit.validate_order(entries[:3], pairs=2, minimum_idle_seconds=10)
    with pytest.raises(ValueError, match="idle"):
        audit.validate_order(
            [entries[0], {**entries[1], "start_monotonic_ns": 14_000_000_000},
             *entries[2:]],
            pairs=2, minimum_idle_seconds=10,
        )


def test_untrimmed_paired_summary_uses_nearest_rank() -> None:
    by_block = {
        "pair-01-baseline": list(range(1, 21)),
        "pair-01-candidate": list(range(2, 22)),
        "pair-02-candidate": list(range(3, 23)),
        "pair-02-baseline": list(range(1, 21)),
    }
    result = audit.summarize(by_block, {}, pairs=2, samples_per_block=20)
    assert result["pooled"]["baseline"]["p50_ns"] == 10
    assert result["pooled"]["candidate"]["p95_ns"] == 21
    assert result["pairs"][0]["p50_delta_percent"] == 10.0
    assert result["pairs"][1]["p50_delta_percent"] == 20.0
    assert result["median_pair_p50_delta_percent"] == 15.0
    assert result["warned_pairs"] == []
    with pytest.raises(ValueError, match="sample count"):
        audit.summarize({**by_block, "pair-01-baseline": [1]}, {}, pairs=2,
                        samples_per_block=20)


def test_environment_warnings_are_recomputed_from_snapshots() -> None:
    start = {
        "host": "host", "kernel": "kernel", "cpu": "cpu", "storage": "disk",
        "governor": "performance", "toolchain": "toolchain", "profiler": "none",
        "swap_pages": 10, "competing_jobs": [], "disk_free_bytes": 2_000_000_000,
    }
    end = {**start, "swap_pages": 12}
    assert audit.validate_environment({
        "start": start, "end": end,
        "invalidators": [], "warnings": ["host swap drift: 2 pages"],
    }) == ["host swap drift: 2 pages"]
    with pytest.raises(ValueError, match="warning"):
        audit.validate_environment({
            "start": start, "end": end, "invalidators": [], "warnings": [],
        })
    with pytest.raises(ValueError, match="drift"):
        audit.validate_environment({
            "start": start, "end": {**end, "cpu": "other"},
            "invalidators": [], "warnings": ["host swap drift: 2 pages"],
        })
