#!/usr/bin/env python3
"""Audit integrated S02 campaign order and the independent paired result."""

from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path

import slice135_python_s02_pair_audit as paired


ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
CAMPAIGN_RUNNER = ROOT / "scripts/slice135_python_s02_integrated_campaign.py"
WHEELS = {
    "baseline": PLAN
    / "results/2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl",
    "candidate": PLAN
    / "results/2026-10-07-python-integrated-candidate/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl",
}


def sha(path: Path) -> str:
    """Hash exact receipt bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_order(entries: list[dict], pairs: list[list[str]], idle_seconds: int) -> None:
    """Verify each timed block followed the frozen alternation and idle gap."""
    expected = [
        (pair_number, position, role, f"pair-{pair_number:02d}-{role}")
        for pair_number, pair in enumerate(pairs, 1)
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
        raise ValueError("integrated S02 block order changed")
    previous_end = None
    for entry in entries:
        if entry.get("exit_code") != 0:
            raise ValueError("integrated S02 block failed")
        started = datetime.fromisoformat(entry["started_utc"])
        ended = datetime.fromisoformat(entry["ended_utc"])
        if ended < started:
            raise ValueError("integrated S02 block time reversed")
        if (
            previous_end is not None
            and (started - previous_end).total_seconds() < idle_seconds
        ):
            raise ValueError("integrated S02 idle interval too short")
        previous_end = ended


def command_value(command: list[str], flag: str) -> str:
    """Extract one unique CLI option from a retained command."""
    if command.count(flag) != 1:
        raise ValueError(f"{flag} missing or repeated")
    return command[command.index(flag) + 1]


def check_output_paths(paths: list[Path], labels: list[str]) -> Path:
    """Verify exact original destinations after a receipt is relocated."""
    if not paths or len(paths) != len(labels):
        raise ValueError("output path count changed")
    parents = {path.parent for path in paths}
    if len(parents) != 1 or [path.name for path in paths] != labels:
        raise ValueError("output paths differ from block labels or original parent")
    return parents.pop()


def audit_integrated(root: Path, protocol_path: Path, paired_audit_path: Path) -> dict:
    """Check campaign order and recompute the complete paired S02 receipt."""
    protocol = json.loads(protocol_path.read_text())
    if (root / "protocol.json").read_bytes() != protocol_path.read_bytes():
        raise ValueError("retained protocol bytes changed")
    manifest_path = root / "run-manifest.json"
    manifest = json.loads(manifest_path.read_text())
    if (
        manifest.get("status") != "FULL_CAMPAIGN_COMPLETE"
        or manifest.get("protocol_sha256") != sha(protocol_path)
        or manifest.get("campaign_runner_sha256") != sha(CAMPAIGN_RUNNER)
        or manifest.get("pair_order") != protocol["pair_order"]
        or manifest.get("samples_per_block") != protocol["samples_per_block"]
        or manifest.get("last_complete_pair") != protocol["pairs"]
    ):
        raise ValueError("campaign completion or exact identity invalid")
    order_path = root / "run-order.jsonl"
    entries = [json.loads(line) for line in order_path.read_text().splitlines()]
    check_order(
        entries,
        protocol["pair_order"],
        protocol["minimum_idle_seconds_between_blocks"],
    )
    output_paths = []
    for entry in entries:
        label, role = entry["label"], entry["role"]
        if entry.get("summary_sha256") != sha(root / label / "summary.json"):
            raise ValueError(f"{label} summary bytes changed")
        command = json.loads((root / f"{label}.driver-command.json").read_text())
        for flag, expected in (
            ("--source-sha", protocol[role]["source_sha"]),
            ("--wheel-sha256", protocol[role]["wheel_sha256"]),
            ("--samples", str(protocol["samples_per_block"])),
            ("--role", role),
            ("--venv-python", manifest[f"{role}_python"]),
        ):
            if command_value(command, flag) != expected:
                raise ValueError(f"{label} {flag} changed")
        output_paths.append(Path(command_value(command, "--output-dir")))
        if (
            Path(command_value(command, "--comparison-protocol")).read_bytes()
            != protocol_path.read_bytes()
        ):
            raise ValueError(f"{label} protocol command changed")
    check_output_paths(output_paths, [entry["label"] for entry in entries])
    recomputed = paired.recompute(
        root, protocol_path, WHEELS["baseline"], WHEELS["candidate"], protocol["pairs"]
    )
    stored = json.loads(paired_audit_path.read_text())
    if recomputed != stored or recomputed["status"] != "FULL_PYTHON_S02_PAIRED":
        raise ValueError("paired independent audit differs from recomputation")
    return {
        "schema_version": 1,
        "status": "VALID_INTEGRATED_S02_ORDER_AND_PAIRED_AUDIT",
        "protocol_sha256": sha(protocol_path),
        "run_manifest_sha256": sha(manifest_path),
        "run_order_sha256": sha(order_path),
        "paired_audit_sha256": sha(paired_audit_path),
        "campaign_runner_sha256": sha(CAMPAIGN_RUNNER),
        "blocks": len(entries),
        "samples_per_version": recomputed["sample_count_per_version"],
        "negative_control_rejected": recomputed["negative_control_rejected"],
    }


def main() -> None:
    """Write one non-overwriting integrated S02 order audit."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--campaign", required=True, type=Path)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--paired-audit", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("refusing to overwrite integrated S02 order audit")
    result = audit_integrated(args.campaign, args.protocol, args.paired_audit)
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
