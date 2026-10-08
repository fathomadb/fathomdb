#!/usr/bin/env python3
"""Independently audit exact-candidate installed-TypeScript S01 pairs."""

from __future__ import annotations

import argparse
from datetime import datetime
import importlib.util
import json
from pathlib import Path
import statistics


ROOT = Path(__file__).resolve().parents[1]
BASE_AUDITOR = ROOT / "scripts/slice135_ts_s01_pair_audit.py"
KINDS = ("text", "vector", "hybrid")


def load_base_auditor():
    """Load existing independent raw checks with isolated integrated identities."""
    specification = importlib.util.spec_from_file_location(
        "slice135_ts_s01_base_for_integrated", BASE_AUDITOR
    )
    assert specification is not None and specification.loader is not None
    base = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(base)
    base.NATIVE_SHA256["candidate"] = (
        "2d26e58cda598d702c3dc333beaeec355e268c093c342fbf1efd26e4ccd5186d"
    )
    base.MODULE_SHA256["candidate"] = (
        "cc0ea0b2ae641aa5162a223985d9afb5ba1233b073899e66a2847f92f36bf432"
    )
    base.ARCHIVES = {
        "baseline": ("baseline-main.tgz", "baseline-platform.tgz"),
        "candidate": ("candidate-main.tgz", "candidate-platform.tgz"),
    }
    return base


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
            or record.get("attempt_status") != "VALID_TS_S01_PAIRED_BLOCK"
            or record.get("invalidators") != []
            or record.get("protocol_sha256") != protocol_sha
        ):
            raise ValueError(f"{name}: run-order validity or protocol differs")


def summarize_pairs(blocks: list[dict], *, size: int, pair_count: int) -> dict:
    """Calculate paired nearest-rank percentage changes from audited blocks."""
    pairs = {}
    for pair in range(1, pair_count + 1):
        pair_blocks = {
            block["role"]: block
            for block in blocks
            if block["size"] == size and block["pair"] == pair
        }
        if set(pair_blocks) != {"baseline", "candidate"}:
            raise ValueError(f"{size}/{pair}: missing baseline or candidate block")
        pairs[pair] = pair_blocks
    result = {}
    for kind in KINDS:
        by_percentile = {}
        for percentile in (50, 95, 99):
            field = f"warm_p{percentile}_ns"
            deltas = []
            clean_deltas = []
            for pair in pairs.values():
                baseline, candidate = pair["baseline"], pair["candidate"]
                before = baseline["cells"][kind][field]
                after = candidate["cells"][kind][field]
                delta = 100 * (after - before) / before
                deltas.append(delta)
                if not baseline["warnings"] and not candidate["warnings"]:
                    clean_deltas.append(delta)
            by_percentile[f"p{percentile}"] = {
                "pair_deltas_pct": [round(value, 4) for value in deltas],
                "median_delta_pct": round(statistics.median(deltas), 4),
                "observed_range_pct": [round(min(deltas), 4), round(max(deltas), 4)],
                "clean_pair_deltas_pct": [round(value, 4) for value in clean_deltas],
                "clean_pair_count": len(clean_deltas),
                "clean_pair_inference": "insufficient"
                if len(clean_deltas) < 3
                else "descriptive_only",
            }
        result[kind] = by_percentile
    return result


def audit_collection(
    root: Path, protocol: Path, baseline_checkout: Path, candidate_checkout: Path
) -> dict:
    """Audit exact archives, raw calls, run order, resources and paired deltas."""
    base = load_base_auditor()
    frozen = base._read(protocol)
    if frozen.get("status") != "FROZEN_TS_S01_PAIRED":
        raise ValueError("integrated TypeScript S01 protocol is not frozen")
    protocol_sha = base._hash(protocol)
    if base._hash(root / "freeze.json") != protocol_sha:
        raise ValueError("archived protocol differs from frozen bytes")
    if base._hash(root / "block-runner.py") != frozen["block_runner_sha256"]:
        raise ValueError("archived TypeScript block runner differs")
    if base._hash(root / "workload.mjs") != frozen["workload_runner_sha256"]:
        raise ValueError("archived TypeScript workload differs")
    archives = base.audit_archives(root / "archives", frozen)
    sources = {
        "baseline": base.audit_source(
            baseline_checkout, frozen["baseline"]["source_sha"]
        ),
        "candidate": base.audit_source(
            candidate_checkout, frozen["candidate"]["source_sha"]
        ),
    }
    expected = base._expected_order(frozen)
    expected_names = {f"{rows}-{pair:02}-{role}" for rows, pair, role in expected}
    actual_names = {
        path.name
        for path in root.iterdir()
        if path.is_dir() and path.name != "archives"
    }
    if actual_names != expected_names:
        raise ValueError("TypeScript S01 block inventory differs from freeze")
    records = [
        json.loads(line) for line in (root / "run-order.jsonl").read_text().splitlines()
    ]
    check_run_order(records, expected, protocol_sha)
    blocks = [
        base.audit_block(
            root / f"{rows}-{pair:02}-{role}",
            frozen,
            protocol_sha,
            sources[role],
            archives[role],
        )
        for rows, pair, role in expected
    ]
    if len({block["raw_sha256"] for block in blocks}) != len(blocks):
        raise ValueError("duplicate raw TypeScript S01 blocks")
    hosts = {(block["host"], block["cpu"], block["governor"]) for block in blocks}
    if len(hosts) != 1:
        raise ValueError("TypeScript S01 host identity changed")
    for previous, current in zip(blocks, blocks[1:]):
        idle = (
            datetime.fromisoformat(current["started_utc"])
            - datetime.fromisoformat(previous["finished_utc"])
        ).total_seconds()
        if idle < frozen["minimum_idle_seconds_between_blocks"]:
            raise ValueError("TypeScript S01 frozen idle interval violated")
    report = {
        "schema_version": 1,
        "status": "PAIRED_DIAGNOSTIC",
        "scope": "integrated-candidate installed TypeScript S01; not full Phase 1",
        "protocol_sha256": protocol_sha,
        "campaign_sha256": base._hash(root / "campaign.py"),
        "block_runner_sha256": base._hash(root / "block-runner.py"),
        "workload_runner_sha256": base._hash(root / "workload.mjs"),
        "archives": archives,
        "sources": sources,
        "valid_blocks": len(blocks),
        "checked_attempts": sum(block["checked_attempts"] for block in blocks),
        "warning_blocks": [block["directory"] for block in blocks if block["warnings"]],
        "sizes": {},
    }
    for size in frozen["rows"]:
        selected = [block for block in blocks if block["size"] == size]
        report["sizes"][str(size)] = {
            "pairs": frozen["pairs_per_size"],
            "checked_attempts": sum(block["checked_attempts"] for block in selected),
            "cells": summarize_pairs(
                selected, size=size, pair_count=frozen["pairs_per_size"]
            ),
            "block_receipts": selected,
        }
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--baseline-checkout", type=Path, required=True)
    parser.add_argument("--candidate-checkout", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit_collection(
        args.root, args.protocol, args.baseline_checkout, args.candidate_checkout
    )
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(result["status"], result["valid_blocks"], result["checked_attempts"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
