#!/usr/bin/env python3
"""Run frozen installed-TypeScript S02 whole-sequence pairs serially."""

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

import slice135_ts_s01_block as s01_block


ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
FREEZE = PLAN / "s02-ts-comparison-protocol.json"
BLOCK = ROOT / "scripts/slice135_ts_s02_block.py"
RUNNER = ROOT / "scripts/slice135_ts_s02.mjs"
S01_HELPER = ROOT / "scripts/slice135_ts_s01.mjs"
PILOT = ROOT / "scripts/slice135_pilot.py"
AUDITOR = ROOT / "scripts/slice135_ts_s02_audit.py"
PAIRED_AUDITOR = ROOT / "scripts/slice135_ts_s02_pair_audit.py"
PAIR_ORDER = (
    ("baseline", "candidate"),
    ("candidate", "baseline"),
    ("baseline", "candidate"),
    ("candidate", "baseline"),
    ("baseline", "candidate"),
)


def sha(path: Path) -> str:
    """Hash exact file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def now() -> str:
    """Return a UTC timestamp for the retained execution order."""
    return datetime.now(timezone.utc).isoformat()


def resolve_freeze(path: Path, expected_sha256: str | None) -> tuple[Path, str]:
    """Require the exact declared protocol bytes before starting a campaign."""
    if not path.is_file():
        raise ValueError("TypeScript S02 freeze missing")
    digest = sha(path)
    if expected_sha256 is not None and digest != expected_sha256:
        raise ValueError("TypeScript S02 freeze bytes changed")
    return path, digest


def planned_blocks(specification: dict) -> list[dict]:
    """Expand and validate the predeclared alternating sample design."""
    if (
        specification.get("schema_version") != 1
        or specification.get("status") != "FROZEN_TS_S02_PAIRED"
    ):
        raise ValueError("TypeScript S02 comparison is not frozen")
    if specification.get("pair_order") != [list(pair) for pair in PAIR_ORDER]:
        raise ValueError("TypeScript S02 pair order changed")
    if (
        specification.get("samples_per_block") != 20
        or specification.get("warmup_per_block") != 1
        or specification.get("pairs") != 5
        or specification.get("total_valid_samples_per_version") != 100
    ):
        raise ValueError("TypeScript S02 sample count changed")
    if specification.get("minimum_idle_seconds_between_blocks", 0) < 20:
        raise ValueError("TypeScript S02 idle interval changed")
    return [
        {"pair": index, "position": position, "role": role, "samples": 20,
         "name": f"pair-{index:02d}-{role}"}
        for index, pair in enumerate(PAIR_ORDER, 1)
        for position, role in enumerate(pair, 1)
    ]


def validate_protocol(specification: dict) -> None:
    """Reject source, pilot, runner or artifact drift before candidate timing."""
    planned_blocks(specification)
    snapshot = specification.get("candidate_product_snapshot")
    if snapshot is not None and (
        not isinstance(snapshot, dict)
        or snapshot.get("source_sha") != specification.get("candidate", {}).get("source_sha")
        or any(
            not isinstance(snapshot.get(key), str)
            or len(snapshot[key]) != 40
            for key in ("rust_crates_tree", "cargo_lock_blob")
        )
    ):
        raise ValueError("TypeScript S02 product snapshot differs from freeze")
    for key, path in (
        ("timed_runner_sha256", RUNNER),
        ("s01_helper_sha256", S01_HELPER),
        ("block_runner_sha256", BLOCK),
        ("pilot_helper_sha256", PILOT),
        ("independent_auditor_sha256", AUDITOR),
        ("paired_auditor_sha256", PAIRED_AUDITOR),
        ("campaign_runner_sha256", Path(__file__)),
    ):
        if specification.get(key) != sha(path):
            raise ValueError(f"TypeScript S02 {key} differs from freeze")
    evidence = PLAN / specification["baseline_noise_evidence"]
    controls = evidence.parent / "negative-controls.json"
    if (
        sha(evidence) != specification.get("baseline_noise_audit_sha256")
        or sha(controls) != specification.get("baseline_noise_negative_controls_sha256")
    ):
        raise ValueError("TypeScript S02 baseline noise evidence changed")
    noise = json.loads(evidence.read_text())
    if (
        noise.get("status") != "CONTROLLED_TS_S02_BASELINE_ONLY_NO_PAIR"
        or noise.get("measured_sequences") != 15
    ):
        raise ValueError("TypeScript S02 baseline noise audit invalid")
    if specification.get("node_version") != "v25.9.0":
        raise ValueError("TypeScript S02 Node runtime changed")


def run_campaign(args: argparse.Namespace) -> None:
    """Retain every block, run order, failure and resolved measurement source."""
    freeze, freeze_sha256 = resolve_freeze(args.freeze, args.freeze_sha256)
    specification = json.loads(freeze.read_text())
    validate_protocol(specification)
    blocks = planned_blocks(specification)
    if subprocess.run(
        [str(args.node), "--version"], capture_output=True, text=True, check=True
    ).stdout.strip() != specification["node_version"]:
        raise ValueError("TypeScript S02 installed Node version differs from freeze")
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
        s01_block.verify_source(artifact["checkout"], frozen["source_sha"])
        s01_block.verify_archives(
            artifact["main"], artifact["platform"], artifact["install"],
            frozen["main_archive_sha256"], frozen["platform_archive_sha256"],
        )
    snapshot = specification.get("candidate_product_snapshot")
    if snapshot is not None:
        checkout = artifacts["candidate"]["checkout"]
        for key, object_name in (
            ("rust_crates_tree", "HEAD:src/rust/crates"),
            ("cargo_lock_blob", "HEAD:Cargo.lock"),
        ):
            actual = subprocess.run(
                ["git", "rev-parse", object_name], cwd=checkout,
                capture_output=True, text=True, check=True,
            ).stdout.strip()
            if actual != snapshot[key]:
                raise ValueError(f"TypeScript S02 candidate product snapshot {key} changed")
    args.output.mkdir(parents=True, exist_ok=False)
    for source, name in (
        (freeze, "freeze.json"),
        (Path(__file__), "campaign.py"),
        (BLOCK, "block-runner.py"),
        (RUNNER, "workload.mjs"),
        (AUDITOR, "pilot-auditor.py"),
        (PAIRED_AUDITOR, "paired-auditor.py"),
    ):
        shutil.copyfile(source, args.output / name)
    run = {
        "schema_version": 1,
        "status": "RUNNING",
        "protocol_sha256": freeze_sha256,
        "campaign_runner_sha256": sha(Path(__file__)),
        "started_utc": now(),
        "pair_order": specification["pair_order"],
        "samples_per_block": 20,
        "last_complete_pair": 0,
        "artifacts": {
            role: {key: str(value) for key, value in artifact.items()}
            for role, artifact in artifacts.items()
        },
        "node": str(args.node),
    }
    manifest_path = args.output / "run-manifest.json"
    manifest_path.write_text(json.dumps(run, indent=2) + "\n")
    order_path = args.output / "run-order.jsonl"
    for index, block in enumerate(blocks):
        if index:
            time.sleep(specification["minimum_idle_seconds_between_blocks"])
        if sha(freeze) != freeze_sha256:
            raise ValueError("TypeScript S02 freeze bytes changed during campaign")
        validate_protocol(json.loads(freeze.read_text()))
        role = block["role"]
        artifact = artifacts[role]
        frozen = specification[role]
        destination = args.output / block["name"]
        command = [
            sys.executable, str(BLOCK),
            "--checkout", str(artifact["checkout"]),
            "--source-sha", frozen["source_sha"],
            "--main-archive", str(artifact["main"]),
            "--platform-archive", str(artifact["platform"]),
            "--main-sha256", frozen["main_archive_sha256"],
            "--platform-sha256", frozen["platform_archive_sha256"],
            "--install-root", str(artifact["install"]),
            "--node", str(args.node),
            "--samples", "20",
            "--comparison-protocol", str(freeze),
            "--role", role,
            "--output-dir", str(destination),
        ]
        started = now()
        try:
            result = subprocess.run(
                command, cwd=ROOT, capture_output=True, text=True,
                check=False, timeout=600,
            )
            stdout, stderr, exit_code = result.stdout, result.stderr, result.returncode
        except subprocess.TimeoutExpired as error:
            stdout = (error.stdout or b"").decode(errors="replace")
            stderr = (error.stderr or b"").decode(errors="replace")
            exit_code = 124
        record = {
            **block,
            "started_utc": started,
            "finished_utc": now(),
            "command": command,
            "exit_code": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "protocol_sha256": freeze_sha256,
            "summary_sha256": sha(destination / "summary.json")
            if (destination / "summary.json").is_file() else None,
        }
        with order_path.open("a") as stream:
            stream.write(json.dumps(record, sort_keys=True) + "\n")
        print(block["name"], exit_code, stdout.strip(), flush=True)
        if exit_code or stdout.strip() != "VALID_TS_S02_PAIRED_BLOCK":
            run["status"] = "INVALID_BLOCK_RETAINED"
            run["updated_utc"] = now()
            manifest_path.write_text(json.dumps(run, indent=2) + "\n")
            raise ValueError(f"{block['name']} invalid; retained in {args.output}")
        if block["position"] == 2:
            run["last_complete_pair"] = block["pair"]
            run["updated_utc"] = now()
            manifest_path.write_text(json.dumps(run, indent=2) + "\n")
    run["status"] = "FULL_CAMPAIGN_COMPLETE"
    run["finished_utc"] = now()
    manifest_path.write_text(json.dumps(run, indent=2) + "\n")


def main() -> None:
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
    args = parser.parse_args()
    for name in ("output", "node", "freeze"):
        setattr(args, name, getattr(args, name).absolute())
    for role in ("baseline", "candidate"):
        for name in ("checkout", "main", "platform", "install"):
            key = f"{role}_{name}"
            setattr(args, key, getattr(args, key).resolve())
    run_campaign(args)


if __name__ == "__main__":
    main()
