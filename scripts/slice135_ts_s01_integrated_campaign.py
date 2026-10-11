#!/usr/bin/env python3
"""Run source-bound frozen installed-TypeScript S01 pairs."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
FREEZE = (
    ROOT
    / "dev/plans/0.8.27/features/slice-135/s01-ts-integrated-comparison-protocol.json"
)
FREEZE_SHA256 = "9fbf31fca79074928c80e1e271e446655923c236e8b4ab3d4900f677f5aa88ec"
BLOCK_RUNNER = ROOT / "scripts/slice135_ts_s01_block.py"
WORKLOAD = ROOT / "scripts/slice135_ts_s01.mjs"
BASELINE_SOURCE = "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
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
    """Expand the frozen TypeScript order, rejecting identity or count drift."""
    if specification.get("status") != "FROZEN_TS_S01_PAIRED":
        raise ValueError("TypeScript S01 protocol is not frozen")
    campaign_hash = specification.get("campaign_runner_sha256")
    if campaign_hash is not None and campaign_hash != sha256(Path(__file__)):
        raise ValueError("campaign runner differs from frozen TypeScript S01")
    if specification.get("baseline", {}).get("source_sha") != BASELINE_SOURCE:
        raise ValueError("baseline source differs from freeze")
    candidate_source = specification.get("candidate", {}).get("source_sha")
    if (
        not isinstance(candidate_source, str)
        or re.fullmatch(r"[0-9a-f]{40}", candidate_source) is None
        or candidate_source == "0" * 40
    ):
        raise ValueError("candidate source identity invalid")
    if (
        specification.get("candidate_product_snapshot", {}).get("source_sha")
        != candidate_source
    ):
        raise ValueError("candidate product snapshot differs from freeze")
    if specification.get("rows") != [32, 256]:
        raise ValueError("TypeScript S01 rows differ from freeze")
    if specification.get("warm_samples_per_cell") != 1000:
        raise ValueError("TypeScript S01 samples differ from p99 floor")
    if specification.get("pairs_per_size") != 5 or specification.get(
        "pair_order_each_size"
    ) != [list(roles) for roles in PAIR_ORDER]:
        raise ValueError("TypeScript S01 pair order differs from freeze")
    if specification.get("minimum_idle_seconds_between_blocks", 0) < 20:
        raise ValueError("TypeScript S01 idle interval differs from freeze")
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
    """Read a clean source checkout identity before any block runs."""
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=checkout, capture_output=True, text=True
    )
    status = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=all"],
        cwd=checkout,
        capture_output=True,
        text=True,
    )
    if head.returncode or status.returncode or status.stdout:
        raise ValueError(f"source checkout unavailable or dirty: {checkout}")
    return head.stdout.strip()


def run(args: argparse.Namespace) -> None:
    """Run each block once, retaining failures and exact command order."""
    freeze = args.freeze
    freeze_sha256 = args.freeze_sha256
    if freeze != FREEZE and freeze_sha256 is None:
        raise ValueError("a refrozen TypeScript S01 protocol requires --freeze-sha256")
    if freeze_sha256 is None:
        freeze_sha256 = FREEZE_SHA256
    if re.fullmatch(r"[0-9a-f]{64}", freeze_sha256) is None:
        raise ValueError("invalid frozen TypeScript S01 protocol SHA-256")
    if sha256(freeze) != freeze_sha256:
        raise ValueError("TypeScript S01 freeze bytes differ from declared hash")
    specification = json.loads(freeze.read_text())
    blocks = planned_blocks(specification)
    if sha256(WORKLOAD) != specification["workload_runner_sha256"]:
        raise ValueError("TypeScript S01 workload differs from freeze")
    if sha256(BLOCK_RUNNER) != specification["block_runner_sha256"]:
        raise ValueError("TypeScript S01 block runner differs from freeze")
    node_version = subprocess.run(
        [str(args.node), "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    if node_version != specification["node_version"]:
        raise ValueError("Node version differs from frozen TypeScript S01")
    artifacts = {
        role: {
            "checkout": getattr(args, f"{role}_checkout"),
            "main": getattr(args, f"{role}_main"),
            "platform": getattr(args, f"{role}_platform"),
            "install": getattr(args, f"{role}_install"),
        }
        for role in ("baseline", "candidate")
    }
    for role, artifact in artifacts.items():
        frozen = specification[role]
        if source_head(artifact["checkout"]) != frozen["source_sha"]:
            raise ValueError(f"{role} source differs from TypeScript S01 freeze")
        if sha256(artifact["main"]) != frozen["main_archive_sha256"]:
            raise ValueError(f"{role} main archive differs from freeze")
        if sha256(artifact["platform"]) != frozen["platform_archive_sha256"]:
            raise ValueError(f"{role} platform archive differs from freeze")
        if not artifact["install"].is_dir():
            raise ValueError(f"{role} installed consumer missing")
    args.output.mkdir(parents=True, exist_ok=False)
    for source, name in (
        (freeze, "freeze.json"),
        (Path(__file__), "campaign.py"),
        (BLOCK_RUNNER, "block-runner.py"),
        (WORKLOAD, "workload.mjs"),
    ):
        shutil.copyfile(source, args.output / name)
    order_path = args.output / "run-order.jsonl"
    for index, block in enumerate(blocks):
        if index:
            time.sleep(specification["minimum_idle_seconds_between_blocks"])
        if sha256(freeze) != freeze_sha256:
            raise ValueError("TypeScript S01 freeze changed during campaign")
        role = block["role"]
        artifact = artifacts[role]
        frozen = specification[role]
        destination = args.output / block["name"]
        command = [
            sys.executable,
            str(BLOCK_RUNNER),
            "--checkout",
            str(artifact["checkout"]),
            "--source-sha",
            frozen["source_sha"],
            "--main-archive",
            str(artifact["main"]),
            "--platform-archive",
            str(artifact["platform"]),
            "--main-sha256",
            frozen["main_archive_sha256"],
            "--platform-sha256",
            frozen["platform_archive_sha256"],
            "--install-root",
            str(artifact["install"]),
            "--node",
            str(args.node),
            "--rows",
            str(block["rows"]),
            "--samples",
            str(block["samples"]),
            "--comparison-protocol",
            str(freeze),
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
            "protocol_sha256": freeze_sha256,
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
            or record.get("attempt_status") != "VALID_TS_S01_PAIRED_BLOCK"
        ):
            raise ValueError(
                f"campaign stopped at {block['name']}; inspect {destination}"
            )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--freeze", type=Path, default=FREEZE)
    parser.add_argument("--freeze-sha256")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--node", type=Path, required=True)
    for role in ("baseline", "candidate"):
        parser.add_argument(f"--{role}-checkout", type=Path, required=True)
        parser.add_argument(f"--{role}-main", type=Path, required=True)
        parser.add_argument(f"--{role}-platform", type=Path, required=True)
        parser.add_argument(f"--{role}-install", type=Path, required=True)
    run(parser.parse_args())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
