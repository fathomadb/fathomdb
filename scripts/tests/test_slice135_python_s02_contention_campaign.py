"""Paired contention run order must alternate the installed versions."""

from __future__ import annotations

from pathlib import Path
import sys


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_contention_campaign as campaign  # noqa: E402


def test_five_pairs_alternate_order_without_dropping_a_role() -> None:
    assert campaign.order_for_pairs(5) == [
        (1, "baseline"), (1, "candidate"),
        (2, "candidate"), (2, "baseline"),
        (3, "baseline"), (3, "candidate"),
        (4, "candidate"), (4, "baseline"),
        (5, "baseline"), (5, "candidate"),
    ]
