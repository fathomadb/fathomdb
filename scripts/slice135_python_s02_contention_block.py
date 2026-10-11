#!/usr/bin/env python3
"""Capture one serial installed-Python S02 contention pilot/timing block."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys

import slice135_pilot as pilot
import slice135_python_s02_contention_audit as audit


ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/slice135_python_s02_contention.py"
AUDITOR = ROOT / "scripts/slice135_python_s02_contention_audit.py"
TIME_FORMAT = (
    "user_s=%U\nsystem_s=%S\npeak_rss_kib=%M\n"
    "major_faults=%F\nswap_events=%W\nexit_code=%x"
)
MIN_DISK_FREE_BYTES = 1_073_741_824


def sha(path: Path) -> str:
    """Hash exact artifact bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def launcher_path(path: Path) -> Path:
    """Keep the virtual-environment interpreter path, including its symlink."""
    return path.absolute()


def validate_identity(protocol: dict, role: str, source_sha: str, wheel_sha: str) -> None:
    """Fail closed if the source or installed wheel differs from its role."""
    if protocol.get("status") != "FROZEN_S02_PYTHON_PAIRED" or role not in {"baseline", "candidate"}:
        raise ValueError("source comparison protocol or role")
    expected = protocol[role]
    if expected.get("source_sha") != source_sha:
        raise ValueError("source SHA differs from role")
    if expected.get("wheel_sha256") != wheel_sha:
        raise ValueError("wheel SHA differs from role")


def git(checkout: Path, *args: str) -> str:
    """Read exact clean product source identity."""
    return subprocess.run(
        ["git", *args], cwd=checkout, capture_output=True, text=True, check=True,
    ).stdout.strip()


def nearest_rank(values: list[int], fraction: float) -> int:
    """Compute the declared nearest-rank percentile."""
    return sorted(values)[math.ceil(len(values) * fraction) - 1]


def run_block(
    *, role: str, checkout: Path, python: Path, wheel: Path,
    comparison_protocol: Path, samples: int, output: Path,
) -> dict:
    """Retain every subprocess, real database and independent audit."""
    if samples < 3:
        raise ValueError("contention block needs at least three measured samples")
    protocol = json.loads(comparison_protocol.read_text())
    source_sha = git(checkout, "rev-parse", "HEAD")
    validate_identity(protocol, role, source_sha, sha(wheel))
    if git(checkout, "status", "--porcelain", "--untracked-files=all"):
        raise ValueError("measured product checkout is dirty")
    if not python.is_file() or not RUNNER.is_file() or not AUDITOR.is_file():
        raise ValueError("installed Python or timing runner missing")
    output.mkdir(parents=True, exist_ok=False)
    identity = {
        "source_sha": source_sha,
        "role": role,
        "wheel_sha256": sha(wheel),
        "comparison_protocol_sha256": sha(comparison_protocol),
        "runner_sha256": sha(RUNNER),
        "auditor_sha256": sha(AUDITOR),
        "block_runner_sha256": sha(Path(__file__)),
    }
    start = pilot.inventory(output)
    env = {**os.environ, "HF_HUB_OFFLINE": "1", "FATHOMDB_EMBED_DEVICE": "cpu"}
    attempts = []
    for number in range(samples + 1):
        label = "warmup" if number == 0 else f"sample-{number:03d}"
        stem = output / label
        database = Path(str(stem) + ".sqlite")
        raw = Path(str(stem) + ".json")
        stdout = Path(str(stem) + ".stdout")
        stderr = Path(str(stem) + ".stderr")
        resource = Path(str(stem) + ".resource")
        independent = Path(str(stem) + ".audit.json")
        command = [
            str(pilot.GNU_TIME), "-o", str(resource), "-f", TIME_FORMAT,
            str(python), str(RUNNER),
            "--database", str(database), "--wheel", str(wheel),
            "--comparison-protocol", str(comparison_protocol),
            "--role", role, "--output", str(raw),
        ]
        (output / f"{label}.command.json").write_text(json.dumps(command) + "\n")
        try:
            child = subprocess.run(
                command, capture_output=True, text=True, check=False,
                env=env, timeout=120,
            )
            stdout.write_text(child.stdout)
            stderr.write_text(child.stderr)
            exit_code = child.returncode
        except subprocess.TimeoutExpired as error:
            stdout.write_bytes(error.stdout or b"")
            stderr.write_bytes(error.stderr or b"")
            exit_code = 124
        attempt: dict = {"label": label, "exit_code": exit_code}
        try:
            if exit_code:
                raise ValueError(f"contention process exited {exit_code}")
            accepted = audit.audit_run(
                raw_path=raw, database=database, wheel=wheel,
                protocol=comparison_protocol, runner=RUNNER,
                stdout=stdout, stderr=stderr, resource=resource,
            )
            independent.write_text(json.dumps(accepted, indent=2) + "\n")
            attempt.update({
                "valid": True,
                "elapsed_ns": accepted["elapsed_ns"],
                "audit_sha256": sha(independent),
                "raw_sha256": sha(raw),
                "database_sha256": sha(database),
                "resource_sha256": sha(resource),
            })
        except (OSError, ValueError, KeyError, IndexError, json.JSONDecodeError) as error:
            attempt.update({"valid": False, "reason": str(error)})
        attempts.append(attempt)
        if not attempt["valid"]:
            break
    end = pilot.inventory(output)
    invalidators = []
    for name in ("host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler"):
        if not start.get(name) or start[name] != end.get(name):
            invalidators.append(f"environment {name} drift")
    if start["competing_jobs"] or end["competing_jobs"]:
        invalidators.append("competing jobs")
    if end["swap_pages"] < start["swap_pages"]:
        invalidators.append("host swap counter decreased")
    if min(start["disk_free_bytes"], end["disk_free_bytes"]) < MIN_DISK_FREE_BYTES:
        invalidators.append("disk pressure")
    warnings = []
    if end["swap_pages"] > start["swap_pages"]:
        warnings.append(f"host swap drift: {end['swap_pages'] - start['swap_pages']} pages")
    for attempt in attempts:
        resource = output / f"{attempt['label']}.resource"
        if not resource.is_file():
            invalidators.append(f"{attempt['label']} resource report missing")
            continue
        report = audit.resource_report(resource)
        if report["swap_events"]:
            invalidators.append(f"{attempt['label']} child swap")
        if report["major_faults"]:
            warnings.append(f"{attempt['label']} child major faults: {report['major_faults']}")
    environment = {
        "start": start, "end": end,
        "invalidators": sorted(set(invalidators)),
        "warnings": sorted(set(warnings)),
    }
    (output / "environment.json").write_text(json.dumps(environment, indent=2) + "\n")
    (output / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
    values = [item["elapsed_ns"] for item in attempts[1:] if item["valid"]]
    valid = (
        len(attempts) == samples + 1
        and all(item["valid"] for item in attempts)
        and not invalidators
    )
    summary = {
        "status": "VALID_S02_PYTHON_CONTENTION_BLOCK" if valid else "INVALID_S02_PYTHON_CONTENTION_BLOCK",
        "identity": identity,
        "samples": samples,
        "warmups": 1,
        "valid_samples": len(values),
        "p50_ns": nearest_rank(values, 0.5) if values else None,
        "p95_ns": nearest_rank(values, 0.95) if len(values) >= 20 else None,
        "min_ns": min(values) if values else None,
        "max_ns": max(values) if values else None,
        "invalidators": sorted(set(invalidators)),
        "warnings": sorted(set(warnings)),
        "attempts_sha256": sha(output / "attempts.json"),
        "environment_sha256": sha(output / "environment.json"),
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--role", required=True, choices=("baseline", "candidate"))
    parser.add_argument("--checkout", required=True, type=Path)
    parser.add_argument("--python", required=True, type=Path)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--comparison-protocol", required=True, type=Path)
    parser.add_argument("--samples", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    result = run_block(
        role=arguments.role, checkout=arguments.checkout.resolve(),
        python=launcher_path(arguments.python), wheel=arguments.wheel.resolve(),
        comparison_protocol=arguments.comparison_protocol.resolve(),
        samples=arguments.samples, output=arguments.output.absolute(),
    )
    print(result["status"])
    if result["status"] != "VALID_S02_PYTHON_CONTENTION_BLOCK":
        sys.exit(1)


if __name__ == "__main__":
    main()
