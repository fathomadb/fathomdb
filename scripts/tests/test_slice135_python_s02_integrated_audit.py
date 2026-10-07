"""Integrated S02 receipt audit checks actual run order and idle time."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_integrated_audit as audit  # noqa: E402


def entries() -> list[dict]:
    return [
        {
            "pair": 1,
            "position": 1,
            "role": "baseline",
            "label": "pair-01-baseline",
            "started_utc": "2026-10-07T00:00:00+00:00",
            "ended_utc": "2026-10-07T00:01:00+00:00",
            "exit_code": 0,
        },
        {
            "pair": 1,
            "position": 2,
            "role": "candidate",
            "label": "pair-01-candidate",
            "started_utc": "2026-10-07T00:01:05+00:00",
            "ended_utc": "2026-10-07T00:02:00+00:00",
            "exit_code": 0,
        },
    ]


def test_order_guard_requires_exact_alternation_and_idle_time() -> None:
    audit.check_order(entries(), [["baseline", "candidate"]], 5)
    changed = list(reversed(entries()))
    with pytest.raises(ValueError, match="order"):
        audit.check_order(changed, [["baseline", "candidate"]], 5)
    changed = deepcopy(entries())
    changed[1]["started_utc"] = "2026-10-07T00:01:04+00:00"
    with pytest.raises(ValueError, match="idle"):
        audit.check_order(changed, [["baseline", "candidate"]], 5)
    changed = deepcopy(entries())
    changed[1]["exit_code"] = 1
    with pytest.raises(ValueError, match="failed"):
        audit.check_order(changed, [["baseline", "candidate"]], 5)


def test_relocated_receipt_keeps_original_command_parent_and_labels() -> None:
    original = Path("/tmp/original-campaign")
    paths = [original / "pair-01-baseline", original / "pair-01-candidate"]
    labels = ["pair-01-baseline", "pair-01-candidate"]
    assert audit.check_output_paths(paths, labels) == original
    with pytest.raises(ValueError, match="output"):
        audit.check_output_paths([paths[0], Path("/tmp/other/pair-01-candidate")], labels)
    with pytest.raises(ValueError, match="output"):
        audit.check_output_paths([paths[0], original / "wrong"], labels)
