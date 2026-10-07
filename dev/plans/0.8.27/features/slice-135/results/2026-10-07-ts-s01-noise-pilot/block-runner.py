#!/usr/bin/env python3
"""Capture one source-bound installed-TypeScript S01 pilot block."""

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
import tarfile


ROOT = Path(__file__).resolve().parents[1]
WORKLOAD = ROOT / "scripts/slice135_ts_s01.mjs"
PILOT_SPEC = importlib.util.spec_from_file_location(
    "slice135_pilot", ROOT / "scripts/slice135_pilot.py"
)
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
MAIN_ARCHIVE_FILES = {
    "package/dist/index.js": "node_modules/fathomdb/dist/index.js",
    "package/package.json": "node_modules/fathomdb/package.json",
}
PLATFORM_ARCHIVE_FILES = {
    "package/fathomdb.linux-x64-gnu.node": (
        "node_modules/fathomdb-linux-x64-gnu/fathomdb.linux-x64-gnu.node"
    ),
    "package/package.json": "node_modules/fathomdb-linux-x64-gnu/package.json",
}


def sha256(data: bytes) -> str:
    """Return a lowercase SHA-256 digest."""
    return hashlib.sha256(data).hexdigest()


def nearest_rank(samples: list[int], quantile: float) -> int:
    """Select the declared nearest-rank percentile without interpolation."""
    if not samples or not 0 < quantile <= 1:
        raise ValueError("nearest-rank input is empty or quantile invalid")
    return sorted(samples)[math.ceil(quantile * len(samples)) - 1]


def _check_attempt(kind: str, attempt: object, eligible_ids: set[str]) -> int:
    if not isinstance(attempt, dict) or attempt.get("semantic_ok") is not True:
        raise ValueError(f"{kind}: semantic failure or malformed attempt")
    elapsed = attempt.get("latency_ns")
    if type(elapsed) is not int or elapsed <= 0:
        raise ValueError(f"{kind}: latency missing or invalid")
    ids = attempt.get("ids")
    branches = attempt.get("branches")
    if (
        not isinstance(ids, list)
        or not isinstance(branches, list)
        or not ids
        or len(ids) != len(branches)
        or any(type(identifier) is not str or identifier not in eligible_ids for identifier in ids)
        or len(set(ids)) != len(ids)
    ):
        raise ValueError(f"{kind}: unknown id, duplicate id or malformed result")
    if kind == "text" and (ids != ["A"] or branches != ["text"]):
        raise ValueError("text result is not the seeded exact match")
    if kind == "vector" and "vector" not in branches:
        raise ValueError("vector branch absent")
    if kind == "hybrid" and (
        "A" not in ids or any(branch not in ("text", "vector") for branch in branches)
    ):
        raise ValueError("hybrid lexical anchor or branch invalid")
    return elapsed


def check_raw(
    raw: dict,
    *,
    size: int,
    samples: int,
    source_sha: str,
    native_sha256: str,
    module_sha256: str | None = None,
) -> dict:
    """Recompute basic result assertions and latency statistics from every call."""
    if not isinstance(raw, dict) or raw.get("schema_version") != 1:
        raise ValueError("raw S01 schema missing or invalid")
    if raw.get("source_sha") != source_sha:
        raise ValueError("source identity mismatch")
    artifact = raw.get("artifact")
    if not isinstance(artifact, dict) or artifact.get("native_sha256") != native_sha256:
        raise ValueError("native artifact identity mismatch")
    if module_sha256 is not None and artifact.get("module_sha256") != module_sha256:
        raise ValueError("installed module identity mismatch")
    if raw.get("corpus_size") != size or raw.get("corpus_sha256") != CORPUS_SHA256[size]:
        raise ValueError("corpus identity mismatch")
    if raw.get("model") != MODEL or raw.get("queries") != QUERIES:
        raise ValueError("model or query configuration mismatch")
    if raw.get("warm_samples_per_cell") != samples or raw.get("semantic_failures") != 0:
        raise ValueError("sample count or semantic failure mismatch")
    cells = raw.get("cells")
    if not isinstance(cells, dict) or set(cells) != set(QUERIES):
        raise ValueError("S01 cell set mismatch")
    eligible_ids = {"A", "B"} | {f"F{index:04}" for index in range(size - 2)}
    summary = {"cells": {}, "checked_attempts": 0}
    for kind in QUERIES:
        cell = cells[kind]
        if not isinstance(cell, dict) or set(cell) != {"session_cold", "warmup", "warm"}:
            raise ValueError(f"{kind}: cell shape mismatch")
        warm = cell["warm"]
        if not isinstance(warm, list) or len(warm) != samples:
            raise ValueError(f"{kind}: sample count mismatch")
        first = _check_attempt(kind, cell["session_cold"], eligible_ids)
        _check_attempt(kind, cell["warmup"], eligible_ids)
        elapsed = [_check_attempt(kind, attempt, eligible_ids) for attempt in warm]
        cell_summary = {
            "session_cold_ns": first,
            "warm_count": len(elapsed),
            "warm_p50_ns": nearest_rank(elapsed, 0.50),
            "warm_p95_ns": nearest_rank(elapsed, 0.95),
            "warm_max_ns": max(elapsed),
        }
        if len(elapsed) >= 1000:
            cell_summary["warm_p99_ns"] = nearest_rank(elapsed, 0.99)
            cell_summary["p99"] = "measured"
        else:
            cell_summary["p99"] = "unsupported_below_1000_samples"
        summary["cells"][kind] = cell_summary
        summary["checked_attempts"] += samples + 2
    return summary


def _git(checkout: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=checkout, capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise ValueError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def verify_source(checkout: Path, source_sha: str) -> dict:
    """Require a clean source checkout at the measured commit."""
    if _git(checkout, "rev-parse", "HEAD") != source_sha:
        raise ValueError("checkout HEAD differs from declared source")
    if _git(checkout, "status", "--porcelain", "--untracked-files=all"):
        raise ValueError("measured source checkout is dirty")
    return {
        "source_sha": source_sha,
        "cargo_lock_sha256": sha256((checkout / "Cargo.lock").read_bytes()),
        "npm_lock_sha256": sha256((checkout / "src/ts/package-lock.json").read_bytes()),
    }


def verify_archives(
    main_archive: Path,
    platform_archive: Path,
    install_root: Path,
    main_sha256: str,
    platform_sha256: str,
) -> dict:
    """Bind installed JS/native bytes to retained npm archive bytes."""
    for archive, expected, names in (
        (main_archive, main_sha256, MAIN_ARCHIVE_FILES),
        (platform_archive, platform_sha256, PLATFORM_ARCHIVE_FILES),
    ):
        if sha256(archive.read_bytes()) != expected:
            raise ValueError(f"package archive SHA-256 mismatch: {archive}")
        with tarfile.open(archive, mode="r:gz") as package:
            for member_name, installed_name in names.items():
                member = package.extractfile(member_name)
                if member is None:
                    raise ValueError(f"archive member missing: {member_name}")
                if sha256(member.read()) != sha256((install_root / installed_name).read_bytes()):
                    raise ValueError(f"installed file differs from archive: {installed_name}")
    module = install_root / MAIN_ARCHIVE_FILES["package/dist/index.js"]
    native = install_root / PLATFORM_ARCHIVE_FILES[
        "package/fathomdb.linux-x64-gnu.node"
    ]
    return {
        "main_archive_sha256": main_sha256,
        "platform_archive_sha256": platform_sha256,
        "module_sha256": sha256(module.read_bytes()),
        "native_sha256": sha256(native.read_bytes()),
    }


def run_block(
    *,
    checkout: Path,
    source_sha: str,
    main_archive: Path,
    platform_archive: Path,
    main_sha256: str,
    platform_sha256: str,
    install_root: Path,
    node: Path,
    size: int,
    samples: int,
    output: Path,
) -> dict:
    """Run one retained S01 baseline noise block with environment/resource checks."""
    if size not in CORPUS_SHA256 or samples < 100:
        raise ValueError("S01 block requires 32/256 rows and at least 100 warm samples")
    if not PILOT.GNU_TIME.is_file():
        raise ValueError("GNU Time resource capture is unavailable")
    source = verify_source(checkout, source_sha)
    archive_identity = verify_archives(
        main_archive,
        platform_archive,
        install_root,
        main_sha256,
        platform_sha256,
    )
    node_version = subprocess.run(
        [str(node), "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    if node_version != "v25.9.0":
        raise ValueError("Node v25.9.0 required for this S01 protocol")
    output.mkdir(parents=True, exist_ok=False)
    (output / "source.json").write_text(json.dumps(source, indent=2, sort_keys=True) + "\n")
    (output / "artifacts.json").write_text(
        json.dumps(archive_identity, indent=2, sort_keys=True) + "\n"
    )
    runner = output / "runner.mjs"
    shutil.copyfile(WORKLOAD, runner)
    runner_sha256 = sha256(runner.read_bytes())
    protocol = {
        "schema_version": 1,
        "status": "BASELINE_TS_S01_NOISE_PILOT_NOT_FROZEN_COMPARISON",
        "source_sha": source_sha,
        "runner_sha256": runner_sha256,
        **archive_identity,
        "corpus_size": size,
        "corpus_sha256": CORPUS_SHA256[size],
        "model": MODEL,
        "queries": QUERIES,
        "warm_samples_per_cell": samples,
        "timing_boundary": "installed TypeScript async call through materialized SearchResult",
        "minimum_disk_free_bytes": MIN_DISK_FREE_BYTES,
        "node_version": node_version,
    }
    (output / "pilot-protocol.json").write_text(
        json.dumps(protocol, indent=2, sort_keys=True) + "\n"
    )
    start = PILOT.inventory(output)
    start["node_version"] = node_version
    resource_path = output / "resource.txt"
    raw_path = output / "raw.json"
    command = [
        str(PILOT.GNU_TIME), "-o", str(resource_path), "-f", PILOT.RESOURCE_FORMAT,
        str(node), str(runner),
        "--rows", str(size), "--samples", str(samples),
        "--install-root", str(install_root),
        "--source-sha", source_sha,
        "--expected-native-sha256", archive_identity["native_sha256"],
        "--output", str(raw_path),
    ]
    (output / "run-command.json").write_text(json.dumps(command, indent=2) + "\n")
    environment = {**os.environ, "FATHOMDB_EMBED_DEVICE": "cpu", "HF_HUB_OFFLINE": "1"}
    invalid: list[str] = []
    try:
        result = subprocess.run(
            command, cwd=ROOT, capture_output=True, text=True,
            env=environment, check=False, timeout=300,
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
    end["node_version"] = node_version
    resources = PILOT.read_resource_report(resource_path)
    invalid.extend(PILOT.environment_invalidators(start, end, MIN_DISK_FREE_BYTES, resources))
    if start["node_version"] != end["node_version"]:
        invalid.append("Node version changed")
    report = {
        "start": start, "end": end, "invalidators": invalid,
        "warnings": PILOT.environment_warnings(start, end, resources),
    }
    (output / "resources.json").write_text(json.dumps(resources, indent=2, sort_keys=True) + "\n")
    summary = None
    if raw_path.is_file():
        try:
            raw = json.loads(raw_path.read_text())
            summary = check_raw(
                raw, size=size, samples=samples, source_sha=source_sha,
                native_sha256=archive_identity["native_sha256"],
                module_sha256=archive_identity["module_sha256"],
            )
            summary["raw_sha256"] = sha256(raw_path.read_bytes())
            summary["runner_sha256"] = runner_sha256
            summary["resource_sha256"] = sha256(resource_path.read_bytes())
            (output / "summary.json").write_text(
                json.dumps(summary, indent=2, sort_keys=True) + "\n"
            )
        except (KeyError, TypeError, ValueError) as error:
            invalid.append(f"raw validation failed: {error}")
    else:
        invalid.append("raw workload output missing")
    report["invalidators"] = invalid
    (output / "environment.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    attempt = {
        "status": "VALID_BASELINE_TS_S01_PILOT_BLOCK" if not invalid else "INVALID_BLOCK",
        "invalidators": invalid,
        "source_sha": source_sha,
        "runner_sha256": runner_sha256,
        "protocol_sha256": sha256((output / "pilot-protocol.json").read_bytes()),
        "summary_sha256": sha256((output / "summary.json").read_bytes()) if summary else None,
    }
    (output / "attempt.json").write_text(json.dumps(attempt, indent=2, sort_keys=True) + "\n")
    return attempt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--main-archive", type=Path, required=True)
    parser.add_argument("--platform-archive", type=Path, required=True)
    parser.add_argument("--main-sha256", required=True)
    parser.add_argument("--platform-sha256", required=True)
    parser.add_argument("--install-root", type=Path, required=True)
    parser.add_argument("--node", type=Path, required=True)
    parser.add_argument("--rows", type=int, choices=(32, 256), required=True)
    parser.add_argument("--samples", type=int, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    attempt = run_block(
        checkout=args.checkout.resolve(), source_sha=args.source_sha,
        main_archive=args.main_archive.resolve(),
        platform_archive=args.platform_archive.resolve(),
        main_sha256=args.main_sha256, platform_sha256=args.platform_sha256,
        install_root=args.install_root.resolve(), node=args.node.resolve(),
        size=args.rows, samples=args.samples, output=args.output_dir.resolve(),
    )
    print(attempt["status"])
    return 0 if attempt["status"] == "VALID_BASELINE_TS_S01_PILOT_BLOCK" else 1


if __name__ == "__main__":
    raise SystemExit(main())
