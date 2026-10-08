#!/usr/bin/env python3
"""Independently check TypeScript S02 block order and observed idle gaps."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path


def sha(path: Path) -> str:
    """Hash exact retained bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_order(rows: list[dict], protocol: dict) -> list[float]:
    """Reject failed, reordered, overlapping, or prematurely started blocks."""
    planned = [
        (pair, position, role, f"pair-{pair:02d}-{role}")
        for pair, roles in enumerate(protocol["pair_order"], 1)
        for position, role in enumerate(roles, 1)
    ]
    observed = [
        (row.get("pair"), row.get("position"), row.get("role"), row.get("name"))
        for row in rows
    ]
    if observed != planned:
        raise ValueError("TypeScript S02 block order changed")
    required_idle = protocol["minimum_idle_seconds_between_blocks"]
    if type(required_idle) is not int or required_idle < 0:
        raise ValueError("TypeScript S02 frozen idle interval invalid")
    gaps = []
    previous_end = None
    for row in rows:
        if row.get("exit_code") != 0:
            raise ValueError(f"TypeScript S02 block failed: {row.get('name')}")
        started = datetime.fromisoformat(row["started_utc"])
        finished = datetime.fromisoformat(row["finished_utc"])
        if (
            started.tzinfo is None or finished.tzinfo is None
            or started.utcoffset() != timezone.utc.utcoffset(started)
            or finished.utcoffset() != timezone.utc.utcoffset(finished)
            or finished <= started
        ):
            raise ValueError(f"TypeScript S02 timestamps invalid: {row['name']}")
        if previous_end is not None:
            gap = (started - previous_end).total_seconds()
            if gap < required_idle:
                raise ValueError(f"TypeScript S02 idle gap too short: {row['name']}")
            gaps.append(gap)
        previous_end = finished
    return gaps


def audit(root: Path, protocol_path: Path) -> dict:
    """Check retained campaign status, protocol bytes and recorded block gaps."""
    protocol = json.loads(protocol_path.read_text())
    protocol_hash = sha(protocol_path)
    if protocol.get("status") != "FROZEN_TS_S02_PAIRED":
        raise ValueError("TypeScript S02 protocol not frozen")
    if sha(root / "freeze.json") != protocol_hash:
        raise ValueError("TypeScript S02 archived freeze changed")
    campaign = json.loads((root / "run-manifest.json").read_text())
    if (
        campaign.get("status") != "FULL_CAMPAIGN_COMPLETE"
        or campaign.get("protocol_sha256") != protocol_hash
        or campaign.get("last_complete_pair") != protocol["pairs"]
    ):
        raise ValueError("TypeScript S02 campaign incomplete")
    order_path = root / "run-order.jsonl"
    rows = [json.loads(line) for line in order_path.read_text().splitlines()]
    gaps = check_order(rows, protocol)
    return {
        "status": "AUDITED_TS_S02_BLOCK_ORDER_AND_IDLE",
        "protocol_sha256": protocol_hash,
        "run_order_sha256": sha(order_path),
        "completed_blocks": len(rows),
        "minimum_required_idle_seconds": protocol["minimum_idle_seconds_between_blocks"],
        "observed_idle_seconds": gaps,
        "minimum_observed_idle_seconds": min(gaps) if gaps else None,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("TypeScript S02 order audit output already exists")
    result = audit(args.root, args.protocol)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"])


if __name__ == "__main__":
    main()
