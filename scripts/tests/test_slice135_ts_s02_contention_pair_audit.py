"""Independent TypeScript contention audit rejects changed pair evidence."""

from __future__ import annotations

from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s02_contention_pair_audit as audit  # noqa: E402


def test_order_requires_all_blocks_and_idle() -> None:
    entries = [
        {
            "pair": pair, "role": role, "block": f"pair-{pair:02d}-{role}",
            "start_monotonic_ns": position * 30_000_000_000,
            "end_monotonic_ns": position * 30_000_000_000 + 5_000_000_000,
            "exit_code": 0,
        }
        for position, (pair, role) in enumerate([
            (1, "baseline"), (1, "candidate"),
            (2, "candidate"), (2, "baseline"),
        ])
    ]
    audit.validate_order(entries, pairs=2, minimum_idle_seconds=20)
    with pytest.raises(ValueError, match="order"):
        audit.validate_order(entries[:3], pairs=2, minimum_idle_seconds=20)
    with pytest.raises(ValueError, match="idle"):
        audit.validate_order([
            entries[0], {**entries[1], "start_monotonic_ns": 24_000_000_000},
            *entries[2:],
        ], pairs=2, minimum_idle_seconds=20)


def test_untrimmed_paired_summary_recomputes_nearest_rank() -> None:
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
    with pytest.raises(ValueError, match="sample count"):
        audit.summarize({**by_block, "pair-01-baseline": [1]}, {},
                        pairs=2, samples_per_block=20)


def test_resource_claim_must_match_raw_gnu_time_report() -> None:
    raw = {
        "user_s": 30.59, "system_s": 10.18, "peak_rss_kib": 486940,
        "fs_inputs": 0, "fs_outputs": 16120, "major_faults": 0, "swap_events": 0,
    }
    claim = {
        "scope": "measured-workload-child-process", "method": "gnu-time",
        "user_cpu_s": 30.59, "system_cpu_s": 10.18, "peak_rss_kib": 486940,
        "fs_inputs": 0, "fs_outputs": 16120, "major_faults": 0,
        "swap_events": 0, "unsupported": [],
    }
    audit.validate_resource_claim(claim, raw)
    with pytest.raises(ValueError, match="resource claim"):
        audit.validate_resource_claim({**claim, "peak_rss_kib": 1}, raw)
