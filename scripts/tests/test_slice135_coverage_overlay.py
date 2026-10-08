"""Checks for workload-versus-test LCOV branch accounting."""

from __future__ import annotations

import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_coverage_overlay as overlay  # noqa: E402


def test_distinguishes_missed_and_unmapped_workload_branches() -> None:
    workload = (
        "SF:/exact/src/rust/crates/fathomdb-engine/src/search.rs\n"
        "DA:10,3\nDA:11,1\n"
        "BRDA:10,0,0,3\nBRDA:10,0,1,2\nBRDA:11,1,0,1\nend_of_record\n"
    )
    tests = (
        "SF:/exact/src/rust/crates/fathomdb-engine/src/search.rs\n"
        "DA:10,1\nDA:11,0\n"
        "BRDA:10,0,0,1\nBRDA:10,0,1,-\nend_of_record\n"
    )
    result = overlay.compare(
        overlay.parse_lcov(workload),
        overlay.parse_lcov(tests),
        prefix="/exact/",
    )["src/rust/crates/fathomdb-engine/src/search.rs"]
    assert result["workload_hit_lines"] == 2
    assert result["workload_hit_branches"] == 3
    assert result["test_hit_workload_branches"] == 1
    assert result["workload_only_branch_ids"] == [[10, 0, 1]]
    assert result["unmapped_workload_branch_ids"] == [[11, 1, 0]]
