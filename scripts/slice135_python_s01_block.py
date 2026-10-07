#!/usr/bin/env python3
"""Capture one source-bound installed-Python S01 baseline pilot block."""

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


ROOT = Path(__file__).resolve().parents[1]
WORKLOAD = ROOT / "scripts/slice135_python_s01.py"
PILOT_PATH = ROOT / "scripts/slice135_pilot.py"
PILOT_SPEC = importlib.util.spec_from_file_location("slice135_pilot", PILOT_PATH)
assert PILOT_SPEC is not None and PILOT_SPEC.loader is not None
PILOT = importlib.util.module_from_spec(PILOT_SPEC)
PILOT_SPEC.loader.exec_module(PILOT)

QUERIES = {
    "text": "brightblue",
    "vector": "boat schedule across water",
    "hybrid": "harbor ferry",
}
CORPUS_SHA256 = {
    32: "e01c7b7772a925ab9c3a80ffb1d20ae1f9338c4fb0501fa16fcc844b871338dd",
    256: "ebbd94f16b40cad184526af3bc2d725b2f433e00683e508adf530d7a925860fb",
}
MODEL = "fathomdb-bge-small-en-v1.5 default embedder"
MIN_DISK_FREE_BYTES = 1_073_741_824


def sha256(data: bytes) -> str:
    """Return a lowercase SHA-256 digest."""
    return hashlib.sha256(data).hexdigest()


def venv_executable(path: Path) -> Path:
    """Make the venv launcher absolute without dereferencing its symlink."""
    return path.absolute()


def nearest_rank(samples: list[int], quantile: float) -> int:
    """Select the declared nearest-rank percentile without interpolation."""
    if not samples or not 0 < quantile <= 1:
        raise ValueError("nearest-rank input is empty or quantile invalid")
    ordered = sorted(samples)
    return ordered[math.ceil(quantile * len(ordered)) - 1]


def _check_attempt(kind: str, attempt: object, eligible_ids: set[str]) -> int:
    if not isinstance(attempt, dict):
        raise ValueError(f"{kind}: attempt is not an object")
    if attempt.get("semantic_ok") is not True:
        raise ValueError(f"{kind}: semantic failure")
    elapsed = attempt.get("latency_ns")
    if type(elapsed) is not int or elapsed <= 0:
        raise ValueError(f"{kind}: elapsed time missing or invalid")
    ids = attempt.get("ids")
    branches = attempt.get("branches")
    if (
        not isinstance(ids, list)
        or not isinstance(branches, list)
        or not ids
        or len(ids) != len(branches)
        or any(
            type(identifier) is not str or identifier not in eligible_ids
            for identifier in ids
        )
    ):
        raise ValueError(f"{kind}: unknown id or malformed result")
    if kind == "text" and (ids != ["A"] or branches != ["text"]):
        raise ValueError("text result is not the seeded exact match")
    if kind == "vector" and "vector" not in branches:
        raise ValueError("vector branch absent")
    if kind == "hybrid" and (
        "A" not in ids or any(branch not in ("text", "vector") for branch in branches)
    ):
        raise ValueError("hybrid anchor or branch invalid")
    return elapsed


def check_raw(
    raw: dict,
    *,
    size: int,
    samples: int,
    source_sha: str,
    wheel_sha256: str,
    corpus_sha256: str,
) -> dict:
    """Recompute every basic result check from the worker's raw output."""
    if not isinstance(raw, dict):
        raise ValueError("raw output is not an object")
    if raw.get("schema_version") != 1 or raw.get("source_sha") != source_sha:
        raise ValueError("source identity or schema mismatch")
    artifact = raw.get("artifact")
    if not isinstance(artifact, dict) or artifact.get("wheel_sha256") != wheel_sha256:
        raise ValueError("wheel identity mismatch")
    if raw.get("corpus_size") != size or raw.get("corpus_sha256") != corpus_sha256:
        raise ValueError("corpus identity mismatch")
    if raw.get("model") != MODEL or raw.get("queries") != QUERIES:
        raise ValueError("model or query configuration mismatch")
    if raw.get("warm_samples_per_cell") != samples:
        raise ValueError("sample count mismatch")
    if raw.get("semantic_failures") != 0:
        raise ValueError("semantic failure reported by worker")
    cells = raw.get("cells")
    if not isinstance(cells, dict) or set(cells) != set(QUERIES):
        raise ValueError("cell set mismatch")
    eligible_ids = {"A", "B"} | {f"F{index:04}" for index in range(size - 2)}
    summary = {"cells": {}, "checked_attempts": 0}
    for kind in QUERIES:
        cell = cells[kind]
        if not isinstance(cell, dict) or set(cell) != {
            "session_cold",
            "warmup",
            "warm",
        }:
            raise ValueError(f"{kind}: cell shape mismatch")
        warm = cell["warm"]
        if not isinstance(warm, list) or len(warm) != samples:
            raise ValueError(f"{kind}: sample count mismatch")
        cold_ns = _check_attempt(kind, cell["session_cold"], eligible_ids)
        _check_attempt(kind, cell["warmup"], eligible_ids)
        warm_ns = [_check_attempt(kind, attempt, eligible_ids) for attempt in warm]
        cell_summary = {
            "session_cold_ns": cold_ns,
            "warm_count": len(warm_ns),
            "warm_p50_ns": nearest_rank(warm_ns, 0.50),
            "warm_p95_ns": nearest_rank(warm_ns, 0.95),
            "warm_max_ns": max(warm_ns),
        }
        if len(warm_ns) >= 1000:
            cell_summary["warm_p99_ns"] = nearest_rank(warm_ns, 0.99)
            cell_summary["p99"] = "measured"
        else:
            cell_summary["p99"] = "unsupported_below_1000_samples"
        summary["cells"][kind] = cell_summary
        summary["checked_attempts"] += samples + 2
    return summary


def _run_git(checkout: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=checkout, capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise ValueError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def _verify_source(checkout: Path, source_sha: str) -> dict:
    resolved = _run_git(checkout, "rev-parse", "HEAD")
    if resolved != source_sha:
        raise ValueError("checkout HEAD differs from declared source")
    if _run_git(checkout, "status", "--porcelain", "--untracked-files=all"):
        raise ValueError("measured source checkout is dirty")
    return {
        "source_sha": source_sha,
        "cargo_lock_sha256": sha256((checkout / "Cargo.lock").read_bytes()),
        "python_project_sha256": sha256(
            (checkout / "src/python/pyproject.toml").read_bytes()
        ),
    }


def validate_comparison_protocol(
    specification: dict,
    *,
    role: str,
    size: int,
    samples: int,
    source_sha: str,
    wheel_sha256: str,
) -> None:
    """Reject a paired block that differs from the frozen S01 subset."""
    if (
        specification.get("schema_version") != 1
        or specification.get("status") != "FROZEN_S01_PYTHON_PAIRED"
    ):
        raise ValueError("paired comparison protocol is not frozen")
    if role not in ("baseline", "candidate"):
        raise ValueError("comparison role must be baseline or candidate")
    artifact = specification.get(role)
    if not isinstance(artifact, dict) or artifact != {
        "source_sha": source_sha,
        "wheel_sha256": wheel_sha256,
    }:
        raise ValueError(f"{role} artifact identity differs from protocol")
    if size not in specification.get("rows", ()):
        raise ValueError("corpus size differs from comparison protocol")
    if specification.get("warm_samples_per_cell") != samples:
        raise ValueError("sample count differs from comparison protocol")
    if (
        specification.get("corpus_sha256_by_size", {}).get(str(size))
        != CORPUS_SHA256[size]
    ):
        raise ValueError("corpus hash differs from comparison protocol")
    if specification.get("workload_runner_sha256") != sha256(WORKLOAD.read_bytes()):
        raise ValueError("workload runner differs from comparison protocol")
    block_hash = specification.get("block_runner_sha256")
    if block_hash is not None and block_hash != sha256(Path(__file__).read_bytes()):
        raise ValueError("block runner differs from comparison protocol")


def run_block(
    *,
    checkout: Path,
    source_sha: str,
    wheel: Path,
    wheel_sha256: str,
    venv_python: Path,
    size: int,
    samples: int,
    output: Path,
    comparison_protocol: Path | None = None,
    role: str = "baseline",
) -> dict:
    """Run one source-bound S01 block and retain output and invalidators."""
    if size not in CORPUS_SHA256 or samples < 100:
        raise ValueError("S01 block requires 32/256 rows and at least 100 warm samples")
    if not PILOT.GNU_TIME.is_file():
        raise ValueError("GNU Time resource capture is unavailable")
    source = _verify_source(checkout, source_sha)
    actual_wheel_hash = sha256(wheel.read_bytes())
    if actual_wheel_hash != wheel_sha256:
        raise ValueError("wheel SHA-256 mismatch")
    if not venv_python.is_file():
        raise ValueError("installed virtual-environment Python is missing")
    comparison_sha256 = None
    if comparison_protocol is not None:
        comparison = json.loads(comparison_protocol.read_text())
        if not isinstance(comparison, dict):
            raise ValueError("comparison protocol is not an object")
        validate_comparison_protocol(
            comparison,
            role=role,
            size=size,
            samples=samples,
            source_sha=source_sha,
            wheel_sha256=wheel_sha256,
        )
        comparison_sha256 = sha256(comparison_protocol.read_bytes())
    elif role != "baseline":
        raise ValueError("candidate run requires a frozen comparison protocol")
    output.mkdir(parents=True, exist_ok=False)
    (output / "source.json").write_text(
        json.dumps(source, indent=2, sort_keys=True) + "\n"
    )
    runner = output / "runner.py"
    shutil.copyfile(WORKLOAD, runner)
    runner_hash = sha256(runner.read_bytes())
    protocol = {
        "schema_version": 1,
        "status": "FROZEN_S01_PYTHON_PAIRED"
        if comparison_sha256
        else "BASELINE_S01_PILOT_NOT_FROZEN_COMPARISON",
        "role": role,
        "comparison_protocol_sha256": comparison_sha256,
        "source_sha": source_sha,
        "wheel_sha256": wheel_sha256,
        "runner_sha256": runner_hash,
        "corpus_size": size,
        "corpus_sha256": CORPUS_SHA256[size],
        "model": MODEL,
        "queries": QUERIES,
        "samples_per_cell": samples,
        "session_cold": "first query after engine reopen; OS/model cache may be warm",
        "warmup_per_cell": 1,
        "warm_boundary": "installed Python call to materialized SearchResult",
        "min_disk_free_bytes": MIN_DISK_FREE_BYTES,
        "timing_mode": "unprofiled",
    }
    (output / "pilot-protocol.json").write_text(
        json.dumps(protocol, indent=2, sort_keys=True) + "\n"
    )
    start = PILOT.inventory(output)
    resource_path = output / "resource.txt"
    raw_path = output / "raw.json"
    measured = [
        str(venv_python),
        str(runner),
        "--rows",
        str(size),
        "--samples",
        str(samples),
        "--wheel",
        str(wheel),
        "--wheel-sha256",
        wheel_sha256,
        "--source-sha",
        source_sha,
        "--output",
        str(raw_path),
    ]
    command = [
        str(PILOT.GNU_TIME),
        "-o",
        str(resource_path),
        "-f",
        PILOT.RESOURCE_FORMAT,
        *measured,
    ]
    (output / "run-command.json").write_text(json.dumps(command, indent=2) + "\n")
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
    environment_report = {
        "start": start,
        "end": end,
        "invalidators": invalid,
        "warnings": PILOT.environment_warnings(start, end, resources),
    }
    (output / "environment.json").write_text(
        json.dumps(environment_report, indent=2, sort_keys=True) + "\n"
    )
    (output / "resources.json").write_text(
        json.dumps(resources, indent=2, sort_keys=True) + "\n"
    )
    summary = None
    if raw_path.is_file():
        try:
            raw = json.loads(raw_path.read_text())
            summary = check_raw(
                raw,
                size=size,
                samples=samples,
                source_sha=source_sha,
                wheel_sha256=wheel_sha256,
                corpus_sha256=CORPUS_SHA256[size],
            )
            summary["raw_sha256"] = sha256(raw_path.read_bytes())
            summary["runner_sha256"] = runner_hash
            summary["resource_sha256"] = (
                sha256(resource_path.read_bytes()) if resource_path.is_file() else None
            )
            (output / "summary.json").write_text(
                json.dumps(summary, indent=2, sort_keys=True) + "\n"
            )
        except (KeyError, TypeError, ValueError) as error:
            invalid.append(f"raw validation failed: {error}")
    else:
        invalid.append("raw workload output missing")
    environment_report["invalidators"] = invalid
    (output / "environment.json").write_text(
        json.dumps(environment_report, indent=2, sort_keys=True) + "\n"
    )
    attempt = {
        "status": (
            "VALID_S01_PAIRED_BLOCK"
            if comparison_sha256
            else "VALID_BASELINE_PILOT_BLOCK"
        )
        if not invalid
        else "INVALID_BLOCK",
        "invalidators": invalid,
        "role": role,
        "comparison_protocol_sha256": comparison_sha256,
        "source_sha": source_sha,
        "wheel_sha256": wheel_sha256,
        "runner_sha256": runner_hash,
        "protocol_sha256": sha256((output / "pilot-protocol.json").read_bytes()),
        "summary_sha256": sha256((output / "summary.json").read_bytes())
        if summary
        else None,
    }
    (output / "attempt.json").write_text(
        json.dumps(attempt, indent=2, sort_keys=True) + "\n"
    )
    return attempt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--venv-python", type=Path, required=True)
    parser.add_argument("--rows", type=int, choices=(32, 256), required=True)
    parser.add_argument("--samples", type=int, required=True)
    parser.add_argument("--comparison-protocol", type=Path)
    parser.add_argument("--role", choices=("baseline", "candidate"), default="baseline")
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    attempt = run_block(
        checkout=args.checkout.resolve(),
        source_sha=args.source_sha,
        wheel=args.wheel.resolve(),
        wheel_sha256=args.wheel_sha256,
        venv_python=venv_executable(args.venv_python),
        size=args.rows,
        samples=args.samples,
        output=args.output_dir.resolve(),
        comparison_protocol=args.comparison_protocol,
        role=args.role,
    )
    print(attempt["status"])
    return 0 if attempt["status"].startswith("VALID_") else 1


if __name__ == "__main__":
    raise SystemExit(main())
