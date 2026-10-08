#!/usr/bin/env python3
"""Capture one S03 installed-Python baseline noise block with resources."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

import slice135_python_s01_block as s01block
import slice135_python_s03_audit as audit


ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/slice135_python_s03.py"
BASELINE_SOURCE = "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
MIN_DISK_FREE_BYTES = 1_073_741_824
PILOT = s01block.PILOT


def digest(path: Path) -> str:
    """Return the SHA-256 of the exact local file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_request(*, size: int, repetitions: int) -> None:
    """Keep the pilot's declared cells and p50/p95 sample floor fixed."""
    if size not in (32, 256) or repetitions < 100:
        raise ValueError(
            "S03 baseline pilot requires 32/256 rows and at least 100 repetitions"
        )


def write_json(path: Path, value: object) -> None:
    """Write a retained block record with stable key ordering."""
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def run_block(
    *,
    checkout: Path,
    wheel: Path,
    wheel_sha256: str,
    venv_python: Path,
    size: int,
    repetitions: int,
    output: Path,
) -> dict:
    """Run one real-database baseline block and keep invalid attempts."""
    validate_request(size=size, repetitions=repetitions)
    if not PILOT.GNU_TIME.is_file():
        raise ValueError("GNU Time resource capture unavailable")
    source = s01block._verify_source(checkout, BASELINE_SOURCE)
    if digest(wheel) != wheel_sha256:
        raise ValueError("baseline wheel hash mismatch")
    if not venv_python.is_file():
        raise ValueError("baseline installed Python missing")
    output.mkdir(parents=True, exist_ok=False)
    write_json(output / "source.json", source)
    protocol = {
        "schema_version": 1,
        "status": "BASELINE_S03_PILOT_NOT_FROZEN_COMPARISON",
        "source_sha": BASELINE_SOURCE,
        "wheel_sha256": wheel_sha256,
        "runner_sha256": digest(RUNNER),
        "auditor_sha256": digest(Path(audit.__file__)),
        "block_runner_sha256": digest(Path(__file__)),
        "size": size,
        "repetitions_per_case": repetitions,
        "cells": [
            "filter",
            "temporal_early",
            "temporal_boundary",
            "temporal_late",
            "graph",
            "evidence",
            f"memory_{size}",
        ],
        "timing_boundary": "installed Python call through materialized observation",
        "min_disk_free_bytes": MIN_DISK_FREE_BYTES,
        "timing_mode": "unprofiled",
    }
    write_json(output / "pilot-protocol.json", protocol)
    start = PILOT.inventory(output)
    resource_path = output / "resource.txt"
    command = [
        str(PILOT.GNU_TIME),
        "-o",
        str(resource_path),
        "-f",
        PILOT.RESOURCE_FORMAT,
        str(venv_python),
        str(RUNNER),
        "--size",
        str(size),
        "--repetitions",
        str(repetitions),
        "--wheel",
        str(wheel),
        "--wheel-sha256",
        wheel_sha256,
        "--source-sha",
        BASELINE_SOURCE,
        "--output",
        str(output / "workload"),
    ]
    write_json(output / "run-command.json", command)
    environment = {
        **os.environ,
        "FATHOMDB_EMBED_DEVICE": "cpu",
        "PYTHONOPTIMIZE": "0",
        "PYTHONDONTWRITEBYTECODE": "1",
    }
    environment.pop("PYTHONPATH", None)
    environment.pop("VIRTUAL_ENV", None)
    invalid: list[str] = []
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            capture_output=True,
            text=True,
            env=environment,
            check=False,
            timeout=300,
        )
        (output / "run.stdout.log").write_text(result.stdout)
        (output / "run.stderr.log").write_text(result.stderr)
        if result.returncode:
            invalid.append(f"workload exited {result.returncode}")
    except subprocess.TimeoutExpired as error:
        (output / "run.stdout.log").write_bytes(error.stdout or b"")
        (output / "run.stderr.log").write_bytes(error.stderr or b"")
        invalid.append("workload timed out after 300 seconds")
    end = PILOT.inventory(output)
    resources = PILOT.read_resource_report(resource_path)
    invalid.extend(
        PILOT.environment_invalidators(start, end, MIN_DISK_FREE_BYTES, resources)
    )
    write_json(output / "resources.json", resources)
    summary = None
    if (output / "workload/raw.json").is_file():
        try:
            summary = audit.audit(
                output / "workload",
                size=size,
                source_sha=BASELINE_SOURCE,
                wheel_sha256=wheel_sha256,
            )
            if any(cell["count"] != repetitions for cell in summary["cases"].values()):
                raise ValueError("audited case count differs from pilot declaration")
            write_json(output / "independent-audit.json", summary)
        except (KeyError, TypeError, ValueError) as error:
            invalid.append(f"independent audit failed: {error}")
    else:
        invalid.append("raw workload output missing")
    warnings = PILOT.environment_warnings(start, end, resources)
    write_json(
        output / "environment.json",
        {
            "start": start,
            "end": end,
            "invalidators": invalid,
            "warnings": warnings,
        },
    )
    attempt = {
        "status": "VALID_BASELINE_S03_PILOT_BLOCK" if not invalid else "INVALID_BLOCK",
        "invalidators": invalid,
        "warnings": warnings,
        "source_sha": BASELINE_SOURCE,
        "wheel_sha256": wheel_sha256,
        "runner_sha256": digest(RUNNER),
        "block_runner_sha256": digest(Path(__file__)),
        "protocol_sha256": digest(output / "pilot-protocol.json"),
        "summary_sha256": digest(output / "independent-audit.json")
        if summary
        else None,
    }
    write_json(output / "attempt.json", attempt)
    return attempt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--venv-python", type=Path, required=True)
    parser.add_argument("--size", type=int, choices=(32, 256), required=True)
    parser.add_argument("--repetitions", type=int, default=100)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    attempt = run_block(**vars(args))
    print(json.dumps({"status": attempt["status"], "output": str(args.output)}))
    return 0 if attempt["status"] == "VALID_BASELINE_S03_PILOT_BLOCK" else 1


if __name__ == "__main__":
    raise SystemExit(main())
