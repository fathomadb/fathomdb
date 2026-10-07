#!/usr/bin/env python3
"""Capture one installed-Python S02-L lifecycle block with full raw receipts."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

import slice135_python_close_memory as lifecycle


ROOT = Path(__file__).resolve().parents[1]
PILOT_SPEC = importlib.util.spec_from_file_location(
    "slice135_pilot", ROOT / "scripts/slice135_pilot.py"
)
assert PILOT_SPEC is not None and PILOT_SPEC.loader is not None
PILOT = importlib.util.module_from_spec(PILOT_SPEC)
PILOT_SPEC.loader.exec_module(PILOT)
RUNNER = ROOT / "scripts/slice135_python_close_memory.py"
MIN_DISK_FREE_BYTES = 1_073_741_824


def sha(path: Path) -> str:
    """Hash exact retained bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nearest_rank(values: list[int], quantile: float) -> int:
    """Return an untrimmed nearest-rank percentile."""
    if not values or not 0 < quantile <= 1:
        raise ValueError("nearest-rank input invalid")
    return sorted(values)[math.ceil(quantile * len(values)) - 1]


def validate_sample(raw: dict, manifest: dict) -> int:
    """Check source, installed wheel, runner and lifecycle semantics."""
    if (
        raw.get("schema_version") != 1
        or raw.get("source_sha") != manifest["source_sha"]
    ):
        raise ValueError("source identity changed")
    if raw.get("runner_sha256") != manifest["runner_sha256"]:
        raise ValueError("runner identity changed")
    artifact = raw.get("artifact")
    if not isinstance(artifact, dict) or any(
        artifact.get(key) != manifest[key] for key in ("wheel_sha256", "native_sha256")
    ):
        raise ValueError("installed artifact identity changed")
    if (
        manifest.get("embed_device")
        and raw.get("host", {}).get("embed_device") != manifest["embed_device"]
    ):
        raise ValueError("embed device changed")
    if (
        manifest.get("embedder_name")
        and raw.get("embedder", {}).get("name") != manifest["embedder_name"]
    ):
        raise ValueError("default embedder changed")
    lifecycle.validate_observation(raw)
    if type(raw.get("open_embed_ns")) is not int or raw["open_embed_ns"] <= 0:
        raise ValueError("open and embed duration invalid")
    return raw["close_ns"]


def source_product_matches(checkout: Path, source_sha: str) -> None:
    """Require current product bytes to match the declared source commit."""
    paths = ("Cargo.toml", "Cargo.lock", "src/rust", "src/python")
    for command in (
        ("git", "cat-file", "-e", f"{source_sha}^{{commit}}"),
        ("git", "diff", "--quiet", source_sha, "HEAD", "--", *paths),
        ("git", "diff", "--quiet", "--", *paths),
        ("git", "diff", "--cached", "--quiet", "--", *paths),
    ):
        if subprocess.run(command, cwd=checkout, check=False).returncode:
            raise ValueError(
                f"product checkout differs from source: {' '.join(command)}"
            )


def run_block(
    *,
    checkout: Path,
    source_sha: str,
    wheel: Path,
    wheel_sha256: str,
    venv_python: Path,
    samples: int,
    output: Path,
    role: str = "baseline",
) -> dict:
    """Run one warmup and all measured cycles in separate Python processes."""
    if role not in ("baseline", "candidate") or samples < 3:
        raise ValueError("role or sample count invalid")
    source_product_matches(checkout, source_sha)
    if sha(wheel) != wheel_sha256:
        raise ValueError("wheel SHA-256 changed")
    if not venv_python.is_file() or not PILOT.GNU_TIME.is_file():
        raise ValueError("installed Python or GNU Time missing")
    with zipfile.ZipFile(wheel) as archive:
        native_sha256 = hashlib.sha256(
            archive.read("fathomdb/_fathomdb.abi3.so")
        ).hexdigest()
    output.mkdir(parents=True, exist_ok=False)
    bundle = output / "bundle"
    bundle.mkdir()
    shutil.copyfile(RUNNER, bundle / RUNNER.name)
    manifest = {
        "schema_version": 1,
        "role": role,
        "source_sha": source_sha,
        "wheel_sha256": wheel_sha256,
        "native_sha256": native_sha256,
        "runner_sha256": sha(bundle / RUNNER.name),
        "block_runner_sha256": sha(Path(__file__)),
        "pilot_helper_sha256": sha(ROOT / "scripts/slice135_pilot.py"),
        "samples": samples,
        "warmups": 1,
        "source_checkout": str(checkout),
        "embed_device": "cpu",
        "embedder_name": "fathomdb-bge-small-en-v1.5",
        "timing_boundary": "Engine.close invocation through return, retaining the Python engine handle",
        "memory_boundary": "fresh process: before open, after default model use, immediate close, 200ms closed idle, handle drop",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    env = os.environ.copy()
    env.pop("PYTHONPATH", None)
    env.pop("VIRTUAL_ENV", None)
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    env["FATHOMDB_EMBED_DEVICE"] = "cpu"
    start = PILOT.inventory(output)
    attempts = []
    for number in range(samples + 1):
        label = "warmup" if number == 0 else f"sample-{number:03d}"
        raw_path = output / f"{label}.json"
        resource_path = output / f"{label}.resource.txt"
        command = [
            str(PILOT.GNU_TIME),
            "-o",
            str(resource_path),
            "-f",
            PILOT.RESOURCE_FORMAT,
            str(venv_python),
            str(bundle / RUNNER.name),
            "--wheel",
            str(wheel),
            "--wheel-sha256",
            wheel_sha256,
            "--source-sha",
            source_sha,
            "--output",
            str(raw_path),
        ]
        (output / f"{label}.command.json").write_text(json.dumps(command) + "\n")
        invalid = []
        try:
            result = subprocess.run(
                command,
                cwd=ROOT,
                capture_output=True,
                text=True,
                env=env,
                check=False,
                timeout=60,
            )
            (output / f"{label}.stdout").write_text(result.stdout)
            (output / f"{label}.stderr").write_text(result.stderr)
            if result.returncode:
                invalid.append(f"process exited {result.returncode}")
        except subprocess.TimeoutExpired as error:
            (output / f"{label}.stdout").write_bytes(error.stdout or b"")
            (output / f"{label}.stderr").write_bytes(error.stderr or b"")
            invalid.append("process timed out after 60 seconds")
        resource = PILOT.read_resource_report(resource_path)
        if not PILOT.complete_child_resources(resource) or resource["swap_events"]:
            invalid.append("child resource report missing or swapped")
        try:
            raw = json.loads(raw_path.read_text())
            close_ns = validate_sample(raw, manifest)
            pss_release_kib = raw["opened"]["pss_kib"] - raw["closed_idle"]["pss_kib"]
        except (OSError, ValueError, KeyError, TypeError) as error:
            invalid.append(f"raw validation failed: {error}")
            close_ns = None
            pss_release_kib = None
        attempts.append(
            {
                "label": label,
                "valid": not invalid,
                "invalidators": invalid,
                "close_ns": close_ns,
                "pss_release_kib": pss_release_kib,
                "raw_sha256": sha(raw_path) if raw_path.is_file() else None,
                "resource": resource,
            }
        )
    end = PILOT.inventory(output)
    resources = [attempt["resource"] for attempt in attempts]
    environment_invalid = sorted(
        {
            reason
            for resource in resources
            for reason in PILOT.environment_invalidators(
                start, end, MIN_DISK_FREE_BYTES, resource
            )
        }
    )
    warnings = sorted(
        {
            warning
            for resource in resources
            for warning in PILOT.environment_warnings(start, end, resource)
        }
    )
    (output / "environment.json").write_text(
        json.dumps(
            {
                "start": start,
                "end": end,
                "invalidators": environment_invalid,
                "warnings": warnings,
            },
            indent=2,
        )
        + "\n"
    )
    (output / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
    invalid = environment_invalid + [
        f"{attempt['label']}: {reason}"
        for attempt in attempts
        for reason in attempt["invalidators"]
    ]
    close_samples = [attempt["close_ns"] for attempt in attempts[1:]]
    release_samples = [attempt["pss_release_kib"] for attempt in attempts[1:]]
    summary = {
        "schema_version": 1,
        "status": "VALID_S02_L_BLOCK" if not invalid else "INVALID_BLOCK",
        "role": role,
        "invalidators": invalid,
        "warnings": warnings,
        "manifest_sha256": sha(output / "manifest.json"),
        "environment_sha256": sha(output / "environment.json"),
        "attempts_sha256": sha(output / "attempts.json"),
        "valid_samples": samples if not invalid else 0,
        "p50_close_ns": nearest_rank(close_samples, 0.5) if not invalid else None,
        "p95_close_ns": nearest_rank(close_samples, 0.95) if not invalid else None,
        "p50_pss_release_kib": nearest_rank(release_samples, 0.5)
        if not invalid
        else None,
        "min_pss_release_kib": min(release_samples) if not invalid else None,
        "max_pss_release_kib": max(release_samples) if not invalid else None,
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> int:
    """Parse a single block request and retain its outcome."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--venv-python", required=True, type=Path)
    parser.add_argument("--samples", type=int, required=True)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--role", choices=("baseline", "candidate"), default="baseline")
    args = parser.parse_args()
    result = run_block(
        checkout=args.checkout.resolve(),
        source_sha=args.source_sha,
        wheel=args.wheel.resolve(),
        wheel_sha256=args.wheel_sha256,
        venv_python=args.venv_python.absolute(),
        samples=args.samples,
        output=args.output_dir.resolve(),
        role=args.role,
    )
    print(result["status"])
    return 0 if result["status"].startswith("VALID_") else 1


if __name__ == "__main__":
    raise SystemExit(main())
