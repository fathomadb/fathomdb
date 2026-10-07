#!/usr/bin/env python3
"""Run the frozen installed-Python S01 comparison on integrated candidate bytes."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
FREEZE = (
    ROOT
    / "dev/plans/0.8.27/features/slice-135/s01-python-integrated-comparison-protocol.json"
)
FREEZE_SHA256 = "750c9fdabcaf941b0f966218569718d0f8f1ea86a2a715316a3483229dc72123"
BLOCK_RUNNER = ROOT / "scripts/slice135_python_s01_block.py"
WORKLOAD = ROOT / "scripts/slice135_python_s01.py"
BASELINE_SOURCE = "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
CANDIDATE_SOURCE = "cdf253cd223a82e954591db532397a3d78a2027a"
PAIR_ORDER = (
    ("baseline", "candidate"),
    ("candidate", "baseline"),
    ("baseline", "candidate"),
    ("candidate", "baseline"),
    ("baseline", "candidate"),
)


def sha256(path: Path) -> str:
    """Hash exact file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def planned_blocks(specification: dict) -> list[dict]:
    """Expand the frozen pair schedule, rejecting changed identity or counts."""
    if specification.get("status") != "FROZEN_S01_PYTHON_PAIRED":
        raise ValueError("S01 protocol is not frozen")
    if specification.get("baseline", {}).get("source_sha") != BASELINE_SOURCE:
        raise ValueError("baseline source differs from integrated S01 freeze")
    if specification.get("candidate", {}).get("source_sha") != CANDIDATE_SOURCE:
        raise ValueError("candidate source differs from integrated S01 freeze")
    snapshot = specification.get("candidate_product_snapshot", {})
    if snapshot.get("source_sha") != CANDIDATE_SOURCE:
        raise ValueError("candidate product snapshot differs from protocol")
    if specification.get("rows") != [32, 256]:
        raise ValueError("S01 rows differ from frozen sizes")
    if specification.get("warm_samples_per_cell") != 1000:
        raise ValueError("S01 samples differ from frozen p99 floor")
    if specification.get("pairs_per_size") != 5 or specification.get(
        "pair_order_each_size"
    ) != [list(roles) for roles in PAIR_ORDER]:
        raise ValueError("S01 pair order differs from frozen alternation")
    if specification.get("minimum_idle_seconds_between_blocks", 0) < 20:
        raise ValueError("S01 pair idle interval differs from freeze")
    return [
        {
            "name": f"{rows}-{pair:02}-{role}",
            "rows": rows,
            "pair": pair,
            "role": role,
            "samples": 1000,
        }
        for rows in (32, 256)
        for pair, roles in enumerate(PAIR_ORDER, 1)
        for role in roles
    ]


def source_head(checkout: Path) -> str:
    """Return the clean checkout identity or fail before measurement."""
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=checkout, capture_output=True, text=True
    )
    if result.returncode:
        raise ValueError(f"cannot read source checkout {checkout}")
    status = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=all"],
        cwd=checkout,
        capture_output=True,
        text=True,
    )
    if status.returncode or status.stdout:
        raise ValueError(f"source checkout is dirty: {checkout}")
    return result.stdout.strip()


def run(args: argparse.Namespace) -> None:
    """Execute each frozen block once and preserve failures without replacement."""
    if sha256(FREEZE) != FREEZE_SHA256:
        raise ValueError("integrated S01 protocol bytes changed after commit")
    specification = json.loads(FREEZE.read_text())
    blocks = planned_blocks(specification)
    if sha256(BLOCK_RUNNER) != specification["block_runner_sha256"]:
        raise ValueError("block runner differs from frozen protocol")
    if sha256(WORKLOAD) != specification["workload_runner_sha256"]:
        raise ValueError("workload differs from frozen protocol")
    artifacts = {
        "baseline": {
            "checkout": args.baseline_checkout,
            "wheel": args.baseline_wheel,
            "python": args.baseline_python,
        },
        "candidate": {
            "checkout": args.candidate_checkout,
            "wheel": args.candidate_wheel,
            "python": args.candidate_python,
        },
    }
    for role, artifact in artifacts.items():
        if source_head(artifact["checkout"]) != specification[role]["source_sha"]:
            raise ValueError(f"{role} checkout source differs from frozen protocol")
        if sha256(artifact["wheel"]) != specification[role]["wheel_sha256"]:
            raise ValueError(f"{role} wheel differs from frozen protocol")
        if not artifact["python"].is_file():
            raise ValueError(f"{role} installed Python is unavailable")
    args.output.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(FREEZE, args.output / "freeze.json")
    shutil.copyfile(Path(__file__), args.output / "campaign.py")
    shutil.copyfile(BLOCK_RUNNER, args.output / "block-runner.py")
    order_path = args.output / "run-order.jsonl"
    for index, block in enumerate(blocks):
        if index:
            time.sleep(specification["minimum_idle_seconds_between_blocks"])
        if sha256(FREEZE) != FREEZE_SHA256:
            raise ValueError("frozen protocol changed during campaign")
        role = block["role"]
        artifact = artifacts[role]
        destination = args.output / block["name"]
        command = [
            sys.executable,
            str(BLOCK_RUNNER),
            "--checkout",
            str(artifact["checkout"]),
            "--source-sha",
            specification[role]["source_sha"],
            "--wheel",
            str(artifact["wheel"]),
            "--wheel-sha256",
            specification[role]["wheel_sha256"],
            "--venv-python",
            str(artifact["python"]),
            "--rows",
            str(block["rows"]),
            "--samples",
            str(block["samples"]),
            "--comparison-protocol",
            str(FREEZE),
            "--role",
            role,
            "--output-dir",
            str(destination),
        ]
        started = datetime.now(timezone.utc).isoformat()
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
        record = {
            **block,
            "started_utc": started,
            "finished_utc": datetime.now(timezone.utc).isoformat(),
            "command": command,
            "exit_code": result.returncode,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "protocol_sha256": FREEZE_SHA256,
        }
        if (destination / "attempt.json").is_file():
            attempt = json.loads((destination / "attempt.json").read_text())
            record["attempt_status"] = attempt.get("status")
            record["invalidators"] = attempt.get("invalidators")
        with order_path.open("a") as stream:
            stream.write(json.dumps(record, sort_keys=True) + "\n")
        print(
            block["name"], result.returncode, record.get("attempt_status"), flush=True
        )
        if (
            result.returncode
            or record.get("attempt_status") != "VALID_S01_PAIRED_BLOCK"
        ):
            raise ValueError(
                f"campaign stopped at {block['name']}; inspect {destination}"
            )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    for role in ("baseline", "candidate"):
        parser.add_argument(f"--{role}-checkout", type=Path, required=True)
        parser.add_argument(f"--{role}-wheel", type=Path, required=True)
        parser.add_argument(f"--{role}-python", type=Path, required=True)
    run(parser.parse_args())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
