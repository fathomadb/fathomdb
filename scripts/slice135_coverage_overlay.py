#!/usr/bin/env python3
"""Compare compiler-instrumented workload and test LCOV source paths."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def parse_lcov(source: str) -> dict[str, dict[str, dict[Any, int]]]:
    """Read exact line and branch counters from LCOV export text."""
    files: dict[str, dict[str, dict[Any, int]]] = {}
    current: dict[str, dict[Any, int]] | None = None
    for record in source.splitlines():
        if record.startswith("SF:"):
            current = files.setdefault(record[3:], {"lines": {}, "branches": {}})
        elif record == "end_of_record":
            current = None
        elif current is not None and record.startswith("DA:"):
            fields = record[3:].split(",")
            line, count = int(fields[0]), int(fields[1])
            current["lines"][line] = current["lines"].get(line, 0) + count
        elif current is not None and record.startswith("BRDA:"):
            fields = record[5:].split(",")
            branch = (int(fields[0]), int(fields[1]), int(fields[2]))
            count = 0 if fields[3] == "-" else int(fields[3])
            current["branches"][branch] = current["branches"].get(branch, 0) + count
    return files


def compare(
    workload: dict[str, dict[str, dict[Any, int]]],
    tests: dict[str, dict[str, dict[Any, int]]],
    *,
    prefix: str,
) -> dict[str, dict[str, Any]]:
    """Keep mapped misses distinct from build-specific unmapped branches."""
    if not prefix.endswith("/"):
        raise ValueError("source prefix must end in a slash")
    results = {}
    for absolute, work in workload.items():
        if not absolute.startswith(prefix):
            continue
        relative = absolute[len(prefix) :]
        if not relative.startswith("src/rust/crates/fathomdb-engine/src/"):
            continue
        test = tests.get(absolute, {"lines": {}, "branches": {}})
        work_lines = work["lines"]
        test_lines = test["lines"]
        work_branches = work["branches"]
        test_branches = test["branches"]
        hit_lines = {key for key, count in work_lines.items() if count > 0}
        test_hit_lines = {key for key, count in test_lines.items() if count > 0}
        hit_branches = {key for key, count in work_branches.items() if count > 0}
        test_hit_branches = {key for key, count in test_branches.items() if count > 0}
        mapped = hit_branches & test_branches.keys()
        results[relative] = {
            "workload_line_ids": len(work_lines),
            "test_line_ids": len(test_lines),
            "workload_hit_lines": len(hit_lines),
            "test_hit_workload_lines": len(hit_lines & test_hit_lines),
            "workload_only_line_ids": sorted(
                (hit_lines & test_lines.keys()) - test_hit_lines
            ),
            "unmapped_workload_line_ids": sorted(hit_lines - test_lines.keys()),
            "workload_branch_ids": len(work_branches),
            "test_branch_ids": len(test_branches),
            "workload_hit_branches": len(hit_branches),
            "mapped_workload_hit_branches": len(mapped),
            "test_hit_workload_branches": len(hit_branches & test_hit_branches),
            "workload_only_branch_ids": [
                list(value) for value in sorted(mapped - test_hit_branches)
            ],
            "unmapped_workload_branch_ids": [
                list(value) for value in sorted(hit_branches - test_branches.keys())
            ],
        }
    return results


def main() -> None:
    """Write an inspectable per-file overlay from two separate LCOV exports."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", type=Path, required=True)
    parser.add_argument("--tests", type=Path, required=True)
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = compare(
        parse_lcov(args.workload.read_text()),
        parse_lcov(args.tests.read_text()),
        prefix=args.prefix,
    )
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"files": len(result), "output": str(args.output)}))


if __name__ == "__main__":
    main()
