#!/usr/bin/env python3
"""Independently audit integrated-candidate installed-Python S01 pairs."""

from __future__ import annotations

import argparse
from datetime import datetime
import json
from pathlib import Path
import statistics

import slice135_python_s01_pair_audit as base


KINDS = ("text", "vector", "hybrid")


def check_campaign_runner(specification: dict, path: Path) -> None:
    """Bind the archived driver when the frozen protocol declares its hash."""
    expected = specification.get("campaign_runner_sha256")
    if expected is not None and base._hash(path) != expected:
        raise ValueError("archived campaign runner differs from frozen protocol")


def check_run_order(
    records: list[dict], expected: list[tuple[int, int, str]], protocol_sha: str
) -> None:
    """Reject missing, reordered or invalid measured blocks."""
    if len(records) != len(expected):
        raise ValueError("run-order block count differs from frozen schedule")
    for record, (rows, pair, role) in zip(records, expected):
        name = f"{rows}-{pair:02}-{role}"
        if record.get("name") != name or (
            record.get("rows"),
            record.get("pair"),
            record.get("role"),
        ) != (rows, pair, role):
            raise ValueError("run-order record differs from frozen order")
        if (
            record.get("samples") != 1000
            or record.get("exit_code") != 0
            or record.get("attempt_status") != "VALID_S01_PAIRED_BLOCK"
            or record.get("invalidators") != []
            or record.get("protocol_sha256") != protocol_sha
        ):
            raise ValueError(f"{name}: run-order validity or protocol differs")


def summarize_pairs(
    blocks: list[dict], *, size: int, pair_count: int, kinds: tuple[str, ...]
) -> dict:
    """Recompute nearest-rank paired percentile deltas from audited blocks."""
    by_pair = {
        pair: {
            role: next(
                (
                    block
                    for block in blocks
                    if (block["size"], block["pair"], block["role"])
                    == (size, pair, role)
                ),
                None,
            )
            for role in ("baseline", "candidate")
        }
        for pair in range(1, pair_count + 1)
    }
    if any(block is None for pair in by_pair.values() for block in pair.values()):
        raise ValueError(f"{size}: missing baseline or candidate pair block")
    summaries = {}
    for kind in kinds:
        percentiles = {}
        for percentile in (50, 95, 99):
            field = f"warm_p{percentile}_ns"
            deltas = []
            clean_deltas = []
            for pair in by_pair.values():
                baseline, candidate = pair["baseline"], pair["candidate"]
                baseline_ns = baseline["cells"][kind][field]
                candidate_ns = candidate["cells"][kind][field]
                delta = 100 * (candidate_ns - baseline_ns) / baseline_ns
                deltas.append(delta)
                if not baseline["warnings"] and not candidate["warnings"]:
                    clean_deltas.append(delta)
            percentiles[f"p{percentile}"] = {
                "pair_deltas_pct": [round(value, 4) for value in deltas],
                "median_delta_pct": round(statistics.median(deltas), 4),
                "observed_range_pct": [round(min(deltas), 4), round(max(deltas), 4)],
                "clean_pair_deltas_pct": [round(value, 4) for value in clean_deltas],
                "clean_pair_count": len(clean_deltas),
                "clean_pair_inference": "insufficient"
                if len(clean_deltas) < 3
                else "descriptive_only",
            }
        summaries[kind] = percentiles
    return summaries


def audit_collection(root: Path, protocol: Path) -> dict:
    """Check exact artifacts, raw blocks, run order and paired summaries."""
    specification = base._read(protocol)
    protocol_sha = base._hash(protocol)
    if specification.get("status") != "FROZEN_S01_PYTHON_PAIRED":
        raise ValueError("integrated S01 protocol is not frozen")
    if base._hash(root / "freeze.json") != protocol_sha:
        raise ValueError("archived frozen protocol bytes differ")
    if base._hash(root / "block-runner.py") != specification["block_runner_sha256"]:
        raise ValueError("archived block runner differs from protocol")
    check_campaign_runner(specification, root / "campaign.py")
    for role in ("baseline", "candidate"):
        if base._hash(root / f"{role}.whl") != specification[role]["wheel_sha256"]:
            raise ValueError(f"{role} wheel bytes differ from frozen protocol")
    expected = base._expected_order(specification)
    directory_names = {path.name for path in root.iterdir() if path.is_dir()}
    expected_names = {f"{rows}-{pair:02}-{role}" for rows, pair, role in expected}
    if directory_names != expected_names:
        raise ValueError("integrated S01 block inventory differs from freeze")
    records = [
        json.loads(line) for line in (root / "run-order.jsonl").read_text().splitlines()
    ]
    check_run_order(records, expected, protocol_sha)
    blocks = [
        base.audit_block(root / f"{rows}-{pair:02}-{role}", specification, protocol_sha)
        for rows, pair, role in expected
    ]
    if len({block["raw_sha256"] for block in blocks}) != len(blocks):
        raise ValueError("duplicate raw block bytes")
    hosts = {(block["host"], block["cpu"], block["governor"]) for block in blocks}
    if len(hosts) != 1:
        raise ValueError("host identity changed between S01 blocks")
    for previous, current in zip(blocks, blocks[1:]):
        idle = (
            datetime.fromisoformat(current["started_utc"])
            - datetime.fromisoformat(previous["finished_utc"])
        ).total_seconds()
        if idle < specification["minimum_idle_seconds_between_blocks"]:
            raise ValueError("S01 blocks violate frozen order or idle interval")
    report = {
        "schema_version": 1,
        "status": "PAIRED_DIAGNOSTIC",
        "scope": "integrated-candidate installed Python S01; not full Phase 1",
        "protocol_sha256": protocol_sha,
        "campaign_sha256": base._hash(root / "campaign.py"),
        "block_runner_sha256": base._hash(root / "block-runner.py"),
        "valid_blocks": len(blocks),
        "checked_attempts": sum(block["checked_attempts"] for block in blocks),
        "host_swap_warning_blocks": base.warning_block_names(blocks, "host swap drift:"),
        "child_major_fault_warning_blocks": base.warning_block_names(
            blocks, "child major faults:"
        ),
        "sizes": {},
    }
    for size in specification["rows"]:
        selected = [block for block in blocks if block["size"] == size]
        report["sizes"][str(size)] = {
            "pairs": specification["pairs_per_size"],
            "checked_attempts": sum(block["checked_attempts"] for block in selected),
            "cells": summarize_pairs(
                selected,
                size=size,
                pair_count=specification["pairs_per_size"],
                kinds=KINDS,
            ),
            "block_receipts": selected,
        }
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("protocol", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    serialized = (
        json.dumps(audit_collection(args.root, args.protocol), indent=2, sort_keys=True)
        + "\n"
    )
    if args.output:
        args.output.write_text(serialized)
    else:
        print(serialized, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
