#!/usr/bin/env python3
"""Recompute the declared installed-Python Phase 1 query-cost proxy."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


S01_NAMES = ("text", "vector", "hybrid")
S03_NAMES = (
    "filter",
    "temporal_early",
    "temporal_boundary",
    "temporal_late",
    "graph",
    "evidence",
)


def sha(path: Path) -> str:
    """Hash the receipt bytes actually read."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def select_s01(warm: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Select 100 evenly spaced calls from a fully audited 1,000-call block."""
    if len(warm) != 1000:
        raise ValueError("S01 warm count differs from declared mix")
    selected = [warm[index] for index in range(0, 1000, 10)]
    if any(item.get("semantic_ok") is not True for item in selected):
        raise ValueError("S01 selected semantic observation failed")
    if any(
        type(item.get("latency_ns")) is not int or item["latency_ns"] <= 0
        for item in selected
    ):
        raise ValueError("S01 selected latency is not positive")
    return selected


def rank_costs(samples_by_path: dict[str, list[int]]) -> list[dict[str, Any]]:
    """Rank equal-frequency paths by every retained selected elapsed sample."""
    if (
        not samples_by_path
        or len({len(values) for values in samples_by_path.values()}) != 1
    ):
        raise ValueError("path sample count differs")
    count = len(next(iter(samples_by_path.values())))
    if count == 0:
        raise ValueError("path sample count is zero")
    if any(
        type(value) is not int or value <= 0
        for values in samples_by_path.values()
        for value in values
    ):
        raise ValueError("path latency must be positive integer")
    total = sum(sum(values) for values in samples_by_path.values())
    ranked = []
    cumulative = 0
    for path, values in sorted(
        samples_by_path.items(), key=lambda item: (-sum(item[1]), item[0])
    ):
        elapsed = sum(values)
        cumulative += elapsed
        ranked.append(
            {
                "path": path,
                "samples": count,
                "total_ns": elapsed,
                "fraction": elapsed / total,
                "cumulative_fraction": cumulative / total,
            }
        )
    return ranked


def prefix_at_least(ranked: list[dict[str, Any]], fraction: float) -> list[str]:
    """Return the smallest sorted prefix reaching the declared cost fraction."""
    if not 0 < fraction <= 1:
        raise ValueError("cost fraction must be in (0, 1]")
    for index, row in enumerate(ranked):
        if row["cumulative_fraction"] >= fraction:
            return [item["path"] for item in ranked[: index + 1]]
    raise ValueError("ranked cost never reached threshold")


def _check_raw(raw: dict, *, source: str, wheel: str, size: int) -> None:
    if (
        raw.get("source_sha") != source
        or raw.get("artifact", {}).get("wheel_sha256") != wheel
    ):
        raise ValueError("mixed path source or wheel differs")
    if raw.get("corpus_size", raw.get("size")) != size:
        raise ValueError("mixed path size differs")


def audit_mix(
    protocol: Path, s01_root: Path, s01_audit: Path, s03_root: Path, s03_audit: Path
) -> dict[str, Any]:
    """Bind two independently audited installed-call subsets and rank costs."""
    frozen = json.loads(protocol.read_text())
    if frozen.get("status") != "FROZEN_PHASE1_PYTHON_QUERY_MIX":
        raise ValueError("Phase 1 query mix is not frozen")
    if sha(Path(__file__)) != frozen["analysis_runner_sha256"]:
        raise ValueError("mixed analysis runner differs from freeze")
    if (
        frozen.get("sizes") != [32, 256]
        or frozen.get("pairs") != 5
        or frozen.get("sample_stride") != 10
    ):
        raise ValueError("mixed schedule differs from freeze")
    audits = {
        "s01": json.loads(s01_audit.read_text()),
        "s03": json.loads(s03_audit.read_text()),
    }
    if (
        audits["s01"].get("protocol_sha256") != frozen["s01_protocol_sha256"]
        or audits["s03"].get("protocol_sha256") != frozen["s03_protocol_sha256"]
    ):
        raise ValueError("component protocol differs from mixed freeze")
    if (
        audits["s01"].get("checked_attempts") != 60120
        or audits["s01"].get("valid_blocks") != 20
    ):
        raise ValueError("S01 independent audit incomplete")
    if (
        audits["s03"].get("status") != "AUDITED_S03_PYTHON_PAIRED_SUBSET"
        or len(audits["s03"].get("blocks", [])) != 20
    ):
        raise ValueError("S03 independent audit incomplete")
    s03_blocks = {block["name"]: block for block in audits["s03"]["blocks"]}
    result: dict[str, Any] = {
        "schema_version": 1,
        "status": "AUDITED_PHASE1_PYTHON_QUERY_COST_PROXY",
        "protocol_sha256": sha(protocol),
        "s01_audit_sha256": sha(s01_audit),
        "s03_audit_sha256": sha(s03_audit),
        "boundaries": "Installed Python materialized calls; S01 and S03 use separately prepared real databases",
        "sizes": {},
    }
    for size in frozen["sizes"]:
        costs = {
            role: {name: [] for name in (*S01_NAMES, *S03_NAMES, f"memory_{size}")}
            for role in ("baseline", "candidate")
        }
        s01_receipts = {
            block["directory"]: block
            for block in audits["s01"]["sizes"][str(size)]["block_receipts"]
        }
        for pair in range(1, frozen["pairs"] + 1):
            for role in ("baseline", "candidate"):
                identity = frozen[role]
                name = f"{size}-{pair:02}-{role}"
                s01_path = s01_root / name / "raw.json"
                if (
                    name not in s01_receipts
                    or sha(s01_path) != s01_receipts[name]["raw_sha256"]
                ):
                    raise ValueError(f"{name}: S01 raw differs from independent audit")
                s01_raw = json.loads(s01_path.read_text())
                _check_raw(
                    s01_raw,
                    source=identity["source_sha"],
                    wheel=identity["wheel_sha256"],
                    size=size,
                )
                for path in S01_NAMES:
                    costs[role][path].extend(
                        item["latency_ns"]
                        for item in select_s01(s01_raw["cells"][path]["warm"])
                    )
                s03_name = f"{size}-pair-{pair:02}-{role}"
                s03_path = s03_root / s03_name / "workload/raw.json"
                if (
                    s03_name not in s03_blocks
                    or sha(s03_path) != s03_blocks[s03_name]["raw_sha256"]
                ):
                    raise ValueError(
                        f"{s03_name}: S03 raw differs from independent audit"
                    )
                s03_raw = json.loads(s03_path.read_text())
                _check_raw(
                    s03_raw,
                    source=identity["source_sha"],
                    wheel=identity["wheel_sha256"],
                    size=size,
                )
                for path in (*S03_NAMES, f"memory_{size}"):
                    observations = s03_raw["cases"][path]
                    if len(observations) != 100 or any(
                        item.get("semantic_ok") is not True for item in observations
                    ):
                        raise ValueError(
                            f"{s03_name}/{path}: S03 count or semantic failure"
                        )
                    costs[role][path].extend(
                        item["latency_ns"] for item in observations
                    )
        ranked = {role: rank_costs(paths) for role, paths in costs.items()}
        result["sizes"][str(size)] = {
            "equal_frequency_calls_per_path_per_version": 500,
            "baseline": ranked["baseline"],
            "candidate": ranked["candidate"],
            "candidate_prefix_80pct": prefix_at_least(ranked["candidate"], 0.8),
        }
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--s01-root", type=Path, required=True)
    parser.add_argument("--s01-audit", type=Path, required=True)
    parser.add_argument("--s03-root", type=Path, required=True)
    parser.add_argument("--s03-audit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit_mix(
        args.protocol, args.s01_root, args.s01_audit, args.s03_root, args.s03_audit
    )
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    for size, row in result["sizes"].items():
        print(size, row["candidate_prefix_80pct"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
