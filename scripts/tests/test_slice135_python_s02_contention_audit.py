"""Independent S02 contention audit must prove overlapping SDK calls."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_contention_audit as audit  # noqa: E402


def intervals() -> tuple[dict, dict]:
    timeline = {
        "writer_calls": [
            {"logical_id": f"contend-{number:02d}", "start_ns": number * 20 + 10,
             "end_ns": number * 20 + 20}
            for number in range(8)
        ],
        "reader_calls": [
            {"start_ns": number * 20 + 15, "end_ns": number * 20 + 25,
             "vector_hits": 2}
            for number in range(8)
        ],
    }
    return timeline, {"reader_cycles": 8, "overlap_cycles": 8}


def test_independent_call_overlap_rejects_claim_without_witness() -> None:
    timeline, observed = intervals()
    assert audit.audit_timeline(timeline, observed) == 8
    changed = deepcopy(timeline)
    for item in changed["writer_calls"]:
        item["start_ns"] += 1000
        item["end_ns"] += 1000
    with pytest.raises(ValueError, match="overlap"):
        audit.audit_timeline(changed, observed)


def test_independent_call_overlap_rejects_writer_identity() -> None:
    timeline, observed = intervals()
    timeline["writer_calls"][3]["logical_id"] = "wrong"
    with pytest.raises(ValueError, match="writer"):
        audit.audit_timeline(timeline, observed)
