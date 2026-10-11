#!/usr/bin/env python3
"""Independently audit the frozen installed-Python S02-L paired campaign."""

from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import statistics

import slice135_python_close_audit as block_audit


ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
FROZEN = PLAN / "s02-python-lifecycle-comparison-protocol.json"
CAMPAIGN = ROOT / "scripts/slice135_python_close_campaign.py"
WHEEL = {
    "baseline": PLAN
    / "results/2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl",
    "candidate": PLAN
    / "results/2026-10-07-python-integrated-candidate/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl",
}
MEMORY = ("rss_kib", "pss_kib", "private_kib")
STATES = ("before", "opened", "closed", "closed_idle", "dropped")


def sha(path: Path) -> str:
    """Hash exact file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_order(entries: list[dict], pair_order: list[list[str]]) -> None:
    """Reject an absent, repeated or reordered version block."""
    expected = [
        (number, position, role, f"{number:02d}-{position:02d}-{role}")
        for number, pair in enumerate(pair_order, 1)
        for position, role in enumerate(pair, 1)
    ]
    observed = [
        (
            entry.get("pair"),
            entry.get("position"),
            entry.get("role"),
            entry.get("label"),
        )
        for entry in entries
    ]
    if observed != expected:
        raise ValueError("S02-L paired block order changed")


def summarize_pair(baseline: dict, candidate: dict) -> dict:
    """Use every valid block sample for two paired percentiles and memory."""
    baseline_close = baseline["raw_close_samples_ns"]
    candidate_close = candidate["raw_close_samples_ns"]
    if len(baseline_close) != len(candidate_close) or not baseline_close:
        raise ValueError("paired sample counts differ")
    base_p50 = block_audit.nearest_rank(baseline_close, 0.5)
    cand_p50 = block_audit.nearest_rank(candidate_close, 0.5)
    base_p95 = block_audit.nearest_rank(baseline_close, 0.95)
    cand_p95 = block_audit.nearest_rank(candidate_close, 0.95)
    return {
        "baseline_close_p50_ns": base_p50,
        "candidate_close_p50_ns": cand_p50,
        "close_p50_delta_percent": block_audit.pair_delta(base_p50, cand_p50),
        "baseline_close_p95_ns": base_p95,
        "candidate_close_p95_ns": cand_p95,
        "close_p95_delta_percent": block_audit.pair_delta(base_p95, cand_p95),
        "baseline_pss_release_p50_kib": block_audit.nearest_rank(
            baseline["raw_pss_release_samples_kib"], 0.5
        ),
        "candidate_pss_release_p50_kib": block_audit.nearest_rank(
            candidate["raw_pss_release_samples_kib"], 0.5
        ),
    }


def collect_memory(directory: Path, samples: int) -> dict:
    """Reopen all raw samples and calculate state distributions."""
    states = {state: {field: [] for field in MEMORY} for state in STATES}
    releases = {field: [] for field in MEMORY}
    for number in range(1, samples + 1):
        raw = json.loads((directory / f"sample-{number:03d}.json").read_text())
        for state in STATES:
            for field in MEMORY:
                states[state][field].append(raw[state][field])
        for field in MEMORY:
            releases[field].append(raw["opened"][field] - raw["closed_idle"][field])
    return {
        "median_state_kib": {
            state: {
                field: block_audit.nearest_rank(states[state][field], 0.5)
                for field in MEMORY
            }
            for state in STATES
        },
        "release_samples_kib": releases,
    }


def pooled(values: list[int]) -> dict:
    """Report supported untrimmed distribution statistics."""
    return {
        "count": len(values),
        "p50": block_audit.nearest_rank(values, 0.5),
        "p95": block_audit.nearest_rank(values, 0.95),
        "min": min(values),
        "max": max(values),
    }


def audit_campaign(directory: Path) -> dict:
    """Verify frozen identity, all raw blocks, declared order and deltas."""
    protocol_path = directory / "protocol.json"
    protocol = json.loads(protocol_path.read_text())
    frozen = json.loads(FROZEN.read_text())
    if protocol != frozen or sha(protocol_path) != sha(FROZEN):
        raise ValueError("frozen protocol changed")
    if protocol["status"] != "FROZEN_S02_L_PYTHON_PAIRED":
        raise ValueError("protocol not frozen")
    manifest = json.loads((directory / "campaign-manifest.json").read_text())
    if (
        manifest["status"] != "PRODUCER_COMPLETE_S02_L_PAIRED"
        or manifest["protocol_sha256"] != sha(FROZEN)
        or manifest["campaign_runner_sha256"] != sha(CAMPAIGN)
    ):
        raise ValueError("campaign source or completion invalid")
    entries = [
        json.loads(line)
        for line in (directory / "run-order.jsonl").read_text().splitlines()
    ]
    check_order(entries, protocol["pair_order"])
    blocks = {}
    memory = {}
    previous_end = None
    for entry in entries:
        label, role = entry["label"], entry["role"]
        if entry["exit_code"] != 0 or entry["summary_sha256"] != sha(
            directory / label / "summary.json"
        ):
            raise ValueError(f"{label} failed or summary changed")
        started = datetime.fromisoformat(entry["started_utc"].replace("Z", "+00:00"))
        ended = datetime.fromisoformat(entry["ended_utc"].replace("Z", "+00:00"))
        if ended < started or (
            previous_end is not None
            and (started - previous_end).total_seconds()
            < protocol["minimum_idle_seconds_between_blocks"]
        ):
            raise ValueError("S02-L block time or idle order invalid")
        previous_end = ended
        block = block_audit.audit_block(directory / label, WHEEL[role])
        if (
            block["role"] != role
            or block["samples"] != protocol["samples_per_block"]
            or block["source_sha"] != protocol[role]["source_sha"]
            or block["wheel_sha256"] != protocol[role]["wheel_sha256"]
            or block["runner_sha256"] != protocol["lifecycle_runner_sha256"]
        ):
            raise ValueError(f"{label} identity or count differs from protocol")
        blocks[label] = block
        memory[label] = collect_memory(directory / label, block["samples"])
    pairs = []
    for number, roles in enumerate(protocol["pair_order"], 1):
        labeled = {
            role: f"{number:02d}-{position:02d}-{role}"
            for position, role in enumerate(roles, 1)
        }
        summary = summarize_pair(
            blocks[labeled["baseline"]], blocks[labeled["candidate"]]
        )
        summary.update(
            {
                "pair": number,
                "baseline_block": labeled["baseline"],
                "candidate_block": labeled["candidate"],
                "warnings": sorted(
                    set(
                        blocks[labeled["baseline"]]["warnings"]
                        + blocks[labeled["candidate"]]["warnings"]
                    )
                ),
            }
        )
        pairs.append(summary)
    by_role = {
        role: [block for label, block in blocks.items() if block["role"] == role]
        for role in ("baseline", "candidate")
    }
    closes = {
        role: [value for block in selected for value in block["raw_close_samples_ns"]]
        for role, selected in by_role.items()
    }
    if any(
        len(values) != protocol["total_valid_samples_per_version"]
        for values in closes.values()
    ):
        raise ValueError("100 valid cycles per version required")
    released = {
        role: {
            field: [
                value
                for label, block in blocks.items()
                if block["role"] == role
                for value in memory[label]["release_samples_kib"][field]
            ]
            for field in MEMORY
        }
        for role in ("baseline", "candidate")
    }
    warned_pairs = [item["pair"] for item in pairs if item["warnings"]]
    clean_pairs = [item for item in pairs if not item["warnings"]]
    return {
        "schema_version": 1,
        "status": "VALID_S02_L_PAIRED_AUDIT",
        "protocol_sha256": sha(FROZEN),
        "campaign_manifest_sha256": sha(directory / "campaign-manifest.json"),
        "run_order_sha256": sha(directory / "run-order.jsonl"),
        "source_sha": {role: protocol[role]["source_sha"] for role in by_role},
        "wheel_sha256": {role: protocol[role]["wheel_sha256"] for role in by_role},
        "samples_per_version": protocol["total_valid_samples_per_version"],
        "close_ns": {role: pooled(values) for role, values in closes.items()},
        "pooled_close_p50_delta_percent": block_audit.pair_delta(
            pooled(closes["baseline"])["p50"], pooled(closes["candidate"])["p50"]
        ),
        "pooled_close_p95_delta_percent": block_audit.pair_delta(
            pooled(closes["baseline"])["p95"], pooled(closes["candidate"])["p95"]
        ),
        "memory_release_kib": {
            role: {field: pooled(values) for field, values in fields.items()}
            for role, fields in released.items()
        },
        "pair_p50_delta_percent": [item["close_p50_delta_percent"] for item in pairs],
        "pair_p95_delta_percent": [item["close_p95_delta_percent"] for item in pairs],
        "pair_p50_delta_median_percent": statistics.median(
            item["close_p50_delta_percent"] for item in pairs
        ),
        "pair_p50_delta_range_percent": [
            min(item["close_p50_delta_percent"] for item in pairs),
            max(item["close_p50_delta_percent"] for item in pairs),
        ],
        "warned_pairs": warned_pairs,
        "clean_pair_p50_delta_range_percent": [
            min(item["close_p50_delta_percent"] for item in clean_pairs),
            max(item["close_p50_delta_percent"] for item in clean_pairs),
        ]
        if clean_pairs
        else None,
        "pairs": pairs,
        "blocks": blocks,
        "memory_by_block": memory,
    }


def main() -> None:
    """Write a non-overwriting paired audit from retained raw files."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--campaign", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("refusing to overwrite paired audit")
    result = audit_campaign(args.campaign)
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
