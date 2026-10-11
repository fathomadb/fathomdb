"""The S02 order receipt must prove the frozen idle interval."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s02_order_audit as audit  # noqa: E402


def sample() -> tuple[list[dict], dict]:
    rows = [
        {
            "name": "pair-01-baseline", "pair": 1, "position": 1,
            "role": "baseline", "exit_code": 0,
            "started_utc": "2026-10-08T01:00:00+00:00",
            "finished_utc": "2026-10-08T01:01:40+00:00",
        },
        {
            "name": "pair-01-candidate", "pair": 1, "position": 2,
            "role": "candidate", "exit_code": 0,
            "started_utc": "2026-10-08T01:02:00+00:00",
            "finished_utc": "2026-10-08T01:03:40+00:00",
        },
    ]
    protocol = {
        "pair_order": [["baseline", "candidate"]],
        "minimum_idle_seconds_between_blocks": 20,
    }
    return rows, protocol


def test_idle_gap_at_exact_minimum_passes() -> None:
    rows, protocol = sample()
    assert audit.check_order(rows, protocol) == [20.0]


def test_short_idle_gap_is_rejected() -> None:
    rows, protocol = sample()
    changed = deepcopy(rows)
    changed[1]["started_utc"] = "2026-10-08T01:01:59+00:00"
    with pytest.raises(ValueError, match="idle gap"):
        audit.check_order(changed, protocol)


def test_changed_order_and_failed_block_are_rejected() -> None:
    rows, protocol = sample()
    changed = deepcopy(rows)
    changed[1]["role"] = "baseline"
    with pytest.raises(ValueError, match="order"):
        audit.check_order(changed, protocol)
    changed = deepcopy(rows)
    changed[1]["exit_code"] = 124
    with pytest.raises(ValueError, match="failed"):
        audit.check_order(changed, protocol)
