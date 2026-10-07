"""S02-L paired audit must preserve order and calculate signed deltas."""

from __future__ import annotations

from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_close_pair_audit as audit  # noqa: E402


def test_pair_summary_uses_all_samples_and_signed_changes() -> None:
    baseline = {"raw_close_samples_ns": [4, 5, 6, 7], "raw_pss_release_samples_kib": [-3] * 4}
    candidate = {"raw_close_samples_ns": [8, 10, 12, 14], "raw_pss_release_samples_kib": [40] * 4}
    summary = audit.summarize_pair(baseline, candidate)
    assert summary["close_p50_delta_percent"] == 100.0
    assert summary["close_p95_delta_percent"] == 100.0
    assert summary["baseline_pss_release_p50_kib"] == -3
    assert summary["candidate_pss_release_p50_kib"] == 40


def test_order_check_rejects_swapped_or_missing_blocks() -> None:
    expected = ["01-01-baseline", "01-02-candidate"]
    audit.check_order(
        [
            {"label": expected[0], "role": "baseline", "pair": 1, "position": 1},
            {"label": expected[1], "role": "candidate", "pair": 1, "position": 2},
        ],
        [["baseline", "candidate"]],
    )
    with pytest.raises(ValueError, match="order"):
        audit.check_order(
            [
                {"label": expected[1], "role": "candidate", "pair": 1, "position": 2},
                {"label": expected[0], "role": "baseline", "pair": 1, "position": 1},
            ],
            [["baseline", "candidate"]],
        )
