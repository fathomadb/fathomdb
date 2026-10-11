#!/usr/bin/env python3
"""Capture one environment-checked installed-Python S02 baseline block."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile


ROOT = Path(__file__).resolve().parents[1]
PILOT_SPEC = importlib.util.spec_from_file_location(
    "slice135_pilot", ROOT / "scripts/slice135_pilot.py"
)
assert PILOT_SPEC is not None and PILOT_SPEC.loader is not None
PILOT = importlib.util.module_from_spec(PILOT_SPEC)
PILOT_SPEC.loader.exec_module(PILOT)

RUNNER = ROOT / "scripts/slice135_python_s02_timing.py"
S01_HELPER = ROOT / "scripts/slice135_python_s01.py"
S02_HELPER = ROOT / "scripts/slice135_python_s02.py"
PILOT_HELPER = ROOT / "scripts/slice135_pilot.py"
MIN_DISK_FREE_BYTES = 1_073_741_824
EXPECTED_COUNTS = {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32}
CORPUS_SHA256 = "77c89858630fd30f5b4ce68748f309047908213e6cbb834b3d812ad87f2be051"
GRAPH_RECORDS_SHA256 = (
    "f62b7379f2d7fa61a6ac20a16d4e4bb4a21462a6ff5a6900aee65e6fd8ed8c4f"
)


def sha(path: Path) -> str:
    """Return the SHA-256 of exact file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nearest_rank_median(samples: list[int]) -> int:
    """Use the lower middle observation for an even sample count."""
    if not samples:
        raise ValueError("median requires a sample")
    return sorted(samples)[(len(samples) - 1) // 2]


def validate_comparison_protocol(
    specification: dict,
    *,
    role: str,
    source_sha: str,
    wheel_sha256: str,
    samples: int,
) -> None:
    """Bind a paired block to the frozen S02 subset before it runs."""
    if (
        specification.get("schema_version") != 1
        or specification.get("status") != "FROZEN_S02_PYTHON_PAIRED"
    ):
        raise ValueError("S02 paired protocol is not frozen")
    if role not in ("baseline", "candidate"):
        raise ValueError("comparison role invalid")
    if specification.get(role) != {
        "source_sha": source_sha,
        "wheel_sha256": wheel_sha256,
    }:
        raise ValueError(f"{role} artifact differs from protocol")
    if specification.get("samples_per_block") != samples:
        raise ValueError("paired sample count differs from protocol")
    if (
        specification.get("corpus_sha256") != CORPUS_SHA256
        or specification.get("graph_records_sha256") != GRAPH_RECORDS_SHA256
    ):
        raise ValueError("corpus or graph fixture differs from protocol")
    for key, path in (
        ("block_runner_sha256", Path(__file__)),
        ("timed_runner_sha256", RUNNER),
        ("s01_helper_sha256", S01_HELPER),
        ("s02_helper_sha256", S02_HELPER),
        ("pilot_helper_sha256", PILOT_HELPER),
    ):
        if specification.get(key) != sha(path):
            raise ValueError(f"{key} differs from protocol")


def _git(checkout: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=checkout, capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise ValueError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def validate_sample(raw: dict, manifest: dict) -> int:
    """Reject a malformed or semantically false whole-sequence observation."""
    if (
        raw.get("schema_version") != 1
        or raw.get("source_sha") != manifest["source_sha"]
    ):
        raise ValueError("source identity changed")
    if (
        raw.get("runner_sha256") != manifest["runner_sha256"]
        or raw.get("s01_helper_sha256") != manifest["s01_helper_sha256"]
        or raw.get("s02_helper_sha256") != manifest["s02_helper_sha256"]
    ):
        raise ValueError("runner or helper identity changed")
    if raw.get("artifact", {}).get("wheel_sha256") != manifest["wheel_sha256"]:
        raise ValueError("wheel identity changed")
    if (
        manifest.get("native_sha256")
        and raw["artifact"].get("native_sha256") != manifest["native_sha256"]
    ):
        raise ValueError("native module identity changed")
    if (
        manifest.get("corpus_sha256")
        and raw.get("corpus_sha256") != manifest["corpus_sha256"]
    ):
        raise ValueError("corpus identity changed")
    if (
        manifest.get("graph_records_sha256")
        and raw.get("graph_records_sha256") != manifest["graph_records_sha256"]
    ):
        raise ValueError("graph fixture identity changed")
    if raw.get("semantic_ok") is not True:
        raise ValueError("producer semantic failure")
    elapsed = raw.get("whole_product_ns")
    verification = raw.get("verification_ns")
    stages = raw.get("stage_ns")
    if (
        type(elapsed) is not int
        or elapsed <= 0
        or type(verification) is not int
        or verification <= 0
        or not isinstance(stages, dict)
        or len(stages) != 18
        or any(type(value) is not int or value <= 0 for value in stages.values())
    ):
        raise ValueError("duration or stage receipt invalid")
    if elapsed < sum(stages.values()):
        raise ValueError("whole product timer shorter than stage sum")
    if raw.get("final_canonical_counts") != EXPECTED_COUNTS:
        raise ValueError("reopened canonical rows changed")
    observed = raw.get("observed")
    if not isinstance(observed, dict):
        raise ValueError("materialized observation missing")
    if observed.get("text") != {"ids": ["A"], "branches": ["text"]}:
        raise ValueError("text anchor changed")
    if not observed.get("vector", {}).get("ids") or "vector" not in observed[
        "vector"
    ].get("branches", []):
        raise ValueError("vector branch missing")
    if "A" not in observed.get("hybrid", {}).get("ids", []):
        raise ValueError("hybrid anchor missing")
    if observed.get("graph", {}).get("target_id") != "s02-claim":
        raise ValueError("graph target changed")
    if observed.get("erasure", {}).get("nodes_excised") != 3:
        raise ValueError("erasure changed")
    if observed.get("after_reopen", {}).get("source_absent") is not True:
        raise ValueError("reopened source remains")
    if observed["after_reopen"].get("readiness") != "ready":
        raise ValueError("reopened projection not ready")
    return elapsed


def run_block(
    *,
    checkout: Path,
    source_sha: str,
    wheel: Path,
    wheel_sha256: str,
    venv_python: Path,
    samples: int,
    output: Path,
    comparison_protocol: Path | None = None,
    role: str = "baseline",
) -> dict:
    """Preserve warm-up, samples, host snapshots and invalid attempts."""
    if samples < 3:
        raise ValueError("S02 noise pilot needs at least three samples per block")
    if _git(checkout, "rev-parse", "HEAD") != source_sha:
        raise ValueError("source checkout differs from declared commit")
    if _git(checkout, "status", "--porcelain", "--untracked-files=all"):
        raise ValueError("measured source checkout is dirty")
    if sha(wheel) != wheel_sha256:
        raise ValueError("wheel SHA-256 mismatch")
    if not venv_python.is_file() or not PILOT.GNU_TIME.is_file():
        raise ValueError("installed Python or GNU Time missing")
    comparison_sha256 = None
    if comparison_protocol is not None:
        specification = json.loads(comparison_protocol.read_text())
        validate_comparison_protocol(
            specification,
            role=role,
            source_sha=source_sha,
            wheel_sha256=wheel_sha256,
            samples=samples,
        )
        comparison_sha256 = sha(comparison_protocol)
    elif role != "baseline":
        raise ValueError("candidate requires a frozen S02 comparison protocol")
    with zipfile.ZipFile(wheel) as archive:
        native_sha = hashlib.sha256(
            archive.read("fathomdb/_fathomdb.abi3.so")
        ).hexdigest()
    output.mkdir(parents=True, exist_ok=False)
    bundle = output / "bundle"
    bundle.mkdir()
    for source in (RUNNER, S01_HELPER, S02_HELPER, PILOT_HELPER):
        shutil.copyfile(source, bundle / source.name)
    manifest = {
        "schema_version": 1,
        "status": "FROZEN_S02_PYTHON_PAIRED"
        if comparison_sha256
        else "BASELINE_ONLY_S02_NOISE_BLOCK",
        "role": role,
        "comparison_protocol_sha256": comparison_sha256,
        "source_sha": source_sha,
        "wheel_sha256": wheel_sha256,
        "native_sha256": native_sha,
        "runner_sha256": sha(bundle / RUNNER.name),
        "s01_helper_sha256": sha(bundle / S01_HELPER.name),
        "s02_helper_sha256": sha(bundle / S02_HELPER.name),
        "pilot_helper_sha256": sha(bundle / PILOT_HELPER.name),
        "corpus_sha256": CORPUS_SHA256,
        "graph_records_sha256": GRAPH_RECORDS_SHA256,
        "warmups": 1,
        "samples": samples,
        "timing_boundary": "installed SDK open through materialized reopened close; independent SQLite verification excluded",
        "source_checkout": str(checkout),
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    environment = os.environ.copy()
    environment.pop("PYTHONPATH", None)
    environment.pop("VIRTUAL_ENV", None)
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    environment["FATHOMDB_EMBED_DEVICE"] = "cpu"
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
            str(venv_python.absolute()),
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
                env=environment,
                check=False,
                timeout=180,
            )
            (output / f"{label}.stdout").write_text(result.stdout)
            (output / f"{label}.stderr").write_text(result.stderr)
            if result.returncode:
                invalid.append(f"process exited {result.returncode}")
        except subprocess.TimeoutExpired as error:
            (output / f"{label}.stdout").write_bytes(error.stdout or b"")
            (output / f"{label}.stderr").write_bytes(error.stderr or b"")
            invalid.append("process timed out after 180 seconds")
        resource = PILOT.read_resource_report(resource_path)
        if not PILOT.complete_child_resources(resource) or resource["swap_events"] > 0:
            invalid.append("child resource report missing or swapped")
        try:
            raw = json.loads(raw_path.read_text())
            elapsed = validate_sample(raw, manifest)
        except (OSError, ValueError, KeyError, TypeError) as error:
            invalid.append(f"raw validation failed: {error}")
            elapsed = None
        attempts.append(
            {
                "label": label,
                "valid": not invalid,
                "invalidators": invalid,
                "whole_product_ns": elapsed,
                "raw_sha256": sha(raw_path) if raw_path.is_file() else None,
                "resource": resource,
            }
        )
    end = PILOT.inventory(output)
    environment_invalid = []
    warnings = []
    for attempt in attempts:
        resource = attempt["resource"]
        environment_invalid.extend(
            PILOT.environment_invalidators(start, end, MIN_DISK_FREE_BYTES, resource)
        )
        warnings.extend(PILOT.environment_warnings(start, end, resource))
    environment_invalid = sorted(set(environment_invalid))
    warnings = sorted(set(warnings))
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
    measured = [attempt["whole_product_ns"] for attempt in attempts[1:]]
    invalid = environment_invalid + [
        f"{attempt['label']}: {reason}"
        for attempt in attempts
        for reason in attempt["invalidators"]
    ]
    summary = {
        "schema_version": 1,
        "status": (
            "VALID_S02_PAIRED_BLOCK"
            if comparison_sha256
            else "VALID_BASELINE_S02_PILOT_BLOCK"
        )
        if not invalid
        else "INVALID_BLOCK",
        "role": role,
        "comparison_protocol_sha256": comparison_sha256,
        "invalidators": invalid,
        "warnings": warnings,
        "manifest_sha256": sha(output / "manifest.json"),
        "environment_sha256": sha(output / "environment.json"),
        "attempts_sha256": sha(output / "attempts.json"),
        "valid_samples": len(measured) if not invalid else 0,
        "median_product_ns": nearest_rank_median(measured) if not invalid else None,
        "min_product_ns": min(measured) if not invalid else None,
        "max_product_ns": max(measured) if not invalid else None,
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--venv-python", required=True, type=Path)
    parser.add_argument("--samples", required=True, type=int)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--comparison-protocol", type=Path)
    parser.add_argument("--role", choices=("baseline", "candidate"), default="baseline")
    args = parser.parse_args()
    summary = run_block(
        checkout=args.checkout.resolve(),
        source_sha=args.source_sha,
        wheel=args.wheel.resolve(),
        wheel_sha256=args.wheel_sha256,
        venv_python=args.venv_python.absolute(),
        samples=args.samples,
        output=args.output_dir.absolute(),
        comparison_protocol=args.comparison_protocol,
        role=args.role,
    )
    print(summary["status"])
    return 0 if summary["status"].startswith("VALID_") else 1


if __name__ == "__main__":
    raise SystemExit(main())
