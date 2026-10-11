#!/usr/bin/env python3
"""Independently audit retained 0.8.26 installed-TypeScript S01 noise blocks."""

from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import tarfile


SOURCE_SHA = "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
MAIN_SHA256 = "90363762405041b11e6dd654da9cb6347695399eb37b81c5c09fde91296ac336"
PLATFORM_SHA256 = "b59b1862b11b8ed3edcdc2e5ee86f9db142268bb5e14571054f6245718fc28f6"
NATIVE_SHA256 = "4a3aef2833060cde858faad689e4c734de31dc70e9dab13a00a6594f694c60ca"
MODULE_SHA256 = "9d86c756e481ee09581ff8c687fd74de8cefbf2ad825a582015cb7b703ea4dc7"
CORPUS_SHA256 = {
    32: "e01c7b7772a925ab9c3a80ffb1d20ae1f9338c4fb0501fa16fcc844b871338dd",
    256: "ebbd94f16b40cad184526af3bc2d725b2f433e00683e508adf530d7a925860fb",
}
QUERIES = {
    "text": "brightblue",
    "vector": "boat schedule across water",
    "hybrid": "harbor ferry",
}
MODEL = "fathomdb-bge-small-en-v1.5 default embedder"
KINDS = ("text", "vector", "hybrid")
SAMPLES = 100
NODE_VERSION = "v25.9.0"


def _read(path: Path) -> dict:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise ValueError(f"{path}: JSON root is not an object")
    return value


def _hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _rank(values: list[int], percentile: int) -> int:
    return sorted(values)[(len(values) * percentile + 99) // 100 - 1]


def _attempt(value: object, kind: str, eligible: set[str]) -> int:
    if not isinstance(value, dict) or value.get("semantic_ok") is not True:
        raise ValueError(f"{kind}: malformed or failed attempt")
    elapsed = value.get("latency_ns")
    ids = value.get("ids")
    branches = value.get("branches")
    if type(elapsed) is not int or elapsed <= 0:
        raise ValueError(f"{kind}: invalid latency")
    if (
        not isinstance(ids, list)
        or not isinstance(branches, list)
        or not ids
        or len(ids) != len(branches)
        or any(
            type(identifier) is not str or identifier not in eligible
            for identifier in ids
        )
        or any(branch not in ("text", "vector") for branch in branches)
        or len(ids) != len(set(ids))
    ):
        raise ValueError(f"{kind}: malformed or ineligible materialized result")
    if kind == "text" and (ids != ["A"] or branches != ["text"]):
        raise ValueError("text anchor result changed")
    if kind == "vector" and ("vector" not in branches or "text" in branches):
        raise ValueError("vector-only result branch changed")
    if kind == "hybrid" and "A" not in ids:
        raise ValueError("hybrid anchor missing")
    return elapsed


def audit_observations(raw: dict, size: int, summary: dict | None = None) -> dict:
    """Recompute materialized result validity and percentiles independently."""
    if raw.get("schema_version") != 1 or raw.get("source_sha") != SOURCE_SHA:
        raise ValueError("raw source identity mismatch")
    artifact = raw.get("artifact")
    if not isinstance(artifact, dict) or artifact.get("native_sha256") != NATIVE_SHA256:
        raise ValueError("raw native artifact identity mismatch")
    if (
        artifact.get("module_sha256") != MODULE_SHA256
        or artifact.get("node_version") != NODE_VERSION
    ):
        raise ValueError("raw installed module or Node identity mismatch")
    if (
        raw.get("corpus_size") != size
        or raw.get("corpus_sha256") != CORPUS_SHA256[size]
    ):
        raise ValueError("raw corpus identity mismatch")
    if raw.get("model") != MODEL or raw.get("queries") != QUERIES:
        raise ValueError("raw model or query mismatch")
    if raw.get("warm_samples_per_cell") != SAMPLES:
        raise ValueError("raw warm sample count mismatch")
    cells = raw.get("cells")
    if not isinstance(cells, dict) or set(cells) != set(KINDS):
        raise ValueError("raw cell set mismatch")
    eligible = {"A", "B", *(f"F{number:04}" for number in range(size - 2))}
    result: dict = {"cells": {}, "checked_attempts": 0, "semantic_failures": 0}
    for kind in KINDS:
        cell = cells[kind]
        if not isinstance(cell, dict) or set(cell) != {
            "session_cold",
            "warmup",
            "warm",
        }:
            raise ValueError(f"{kind}: cell shape mismatch")
        warm = cell["warm"]
        if not isinstance(warm, list) or len(warm) != SAMPLES:
            raise ValueError(f"{kind}: warm sample count mismatch")
        cold = _attempt(cell["session_cold"], kind, eligible)
        _attempt(cell["warmup"], kind, eligible)
        durations = [_attempt(attempt, kind, eligible) for attempt in warm]
        result["cells"][kind] = {
            "session_cold_ns": cold,
            "warm_count": len(durations),
            "warm_p50_ns": _rank(durations, 50),
            "warm_p95_ns": _rank(durations, 95),
            "warm_max_ns": max(durations),
            "p99": "unsupported_below_1000_samples",
        }
        result["checked_attempts"] += SAMPLES + 2
    if raw.get("semantic_failures") != 0:
        raise ValueError("worker semantic failure count differs from observations")
    started = datetime.fromisoformat(raw["started_utc"])
    finished = datetime.fromisoformat(raw["finished_utc"])
    if started.tzinfo is None or finished <= started:
        raise ValueError("raw timestamps are missing or reversed")
    result["started_utc"] = raw["started_utc"]
    result["finished_utc"] = raw["finished_utc"]
    result["elapsed_block_s"] = round((finished - started).total_seconds(), 6)
    if summary is not None:
        if summary.get("checked_attempts") != result["checked_attempts"]:
            raise ValueError("summary checked-attempt count mismatch")
        reported = summary.get("cells")
        if not isinstance(reported, dict) or set(reported) != set(KINDS):
            raise ValueError("summary cell set mismatch")
        for kind in KINDS:
            for field, expected in result["cells"][kind].items():
                if reported[kind].get(field) != expected:
                    label = "percentile" if field.startswith("warm_p") else "summary"
                    raise ValueError(f"{kind}: {label} mismatch for {field}")
    return result


def audit_archives(main: Path, platform: Path) -> dict:
    """Bind independent archive contents to the installed bytes in receipts."""
    if _hash(main) != MAIN_SHA256 or _hash(platform) != PLATFORM_SHA256:
        raise ValueError("npm archive hash mismatch")
    for archive, member_name, expected in (
        (main, "package/dist/index.js", MODULE_SHA256),
        (platform, "package/fathomdb.linux-x64-gnu.node", NATIVE_SHA256),
    ):
        with tarfile.open(archive, mode="r:gz") as package:
            member = package.extractfile(member_name)
            if member is None or hashlib.sha256(member.read()).hexdigest() != expected:
                raise ValueError(f"npm archive content mismatch: {member_name}")
    return {
        "main_archive_sha256": MAIN_SHA256,
        "platform_archive_sha256": PLATFORM_SHA256,
        "module_sha256": MODULE_SHA256,
        "native_sha256": NATIVE_SHA256,
    }


def audit_source(checkout: Path) -> dict:
    """Check baseline source and lockfiles independently of producer receipts."""
    for arguments, expected in (
        (["rev-parse", "HEAD"], SOURCE_SHA),
        (["status", "--porcelain", "--untracked-files=all"], ""),
    ):
        result = subprocess.run(
            ["git", *arguments],
            cwd=checkout,
            capture_output=True,
            text=True,
            check=True,
        )
        if result.stdout.strip() != expected:
            raise ValueError("baseline checkout source or cleanliness differs")
    return {
        "source_sha": SOURCE_SHA,
        "cargo_lock_sha256": _hash(checkout / "Cargo.lock"),
        "npm_lock_sha256": _hash(checkout / "src/ts/package-lock.json"),
    }


def audit_block(directory: Path, size: int, source_identity: dict) -> dict:
    """Verify a retained pilot block's hashes, environment and all calls."""
    attempt = _read(directory / "attempt.json")
    protocol = _read(directory / "pilot-protocol.json")
    source = _read(directory / "source.json")
    artifacts = _read(directory / "artifacts.json")
    environment = _read(directory / "environment.json")
    resources = _read(directory / "resources.json")
    raw = _read(directory / "raw.json")
    summary = _read(directory / "summary.json")
    if (
        attempt.get("status") != "VALID_BASELINE_TS_S01_PILOT_BLOCK"
        or attempt.get("invalidators") != []
        or source != source_identity
    ):
        raise ValueError(f"{directory.name}: block status or source invalid")
    for field, filename in (
        ("protocol_sha256", "pilot-protocol.json"),
        ("runner_sha256", "runner.mjs"),
        ("summary_sha256", "summary.json"),
    ):
        if attempt.get(field) != _hash(directory / filename):
            raise ValueError(f"{directory.name}: {field} does not match bytes")
    if protocol.get("runner_sha256") != _hash(directory / "runner.mjs"):
        raise ValueError(f"{directory.name}: protocol runner hash mismatch")
    if summary.get("runner_sha256") != _hash(directory / "runner.mjs"):
        raise ValueError(f"{directory.name}: summary runner hash mismatch")
    if summary.get("raw_sha256") != _hash(directory / "raw.json"):
        raise ValueError(f"{directory.name}: raw hash mismatch")
    if summary.get("resource_sha256") != _hash(directory / "resource.txt"):
        raise ValueError(f"{directory.name}: child resource hash mismatch")
    expected = {
        "source_sha": SOURCE_SHA,
        "main_archive_sha256": MAIN_SHA256,
        "platform_archive_sha256": PLATFORM_SHA256,
        "module_sha256": MODULE_SHA256,
        "native_sha256": NATIVE_SHA256,
        "corpus_size": size,
        "corpus_sha256": CORPUS_SHA256[size],
        "model": MODEL,
        "queries": QUERIES,
        "warm_samples_per_cell": SAMPLES,
        "node_version": NODE_VERSION,
    }
    if any(protocol.get(key) != value for key, value in expected.items()):
        raise ValueError(f"{directory.name}: protocol differs from expected fixture")
    if any(
        artifacts.get(key) != value
        for key, value in expected.items()
        if key
        in (
            "main_archive_sha256",
            "platform_archive_sha256",
            "module_sha256",
            "native_sha256",
        )
    ):
        raise ValueError(f"{directory.name}: artifact receipt mismatch")
    if environment.get("invalidators") != []:
        raise ValueError(f"{directory.name}: environment invalidators")
    before, after = environment.get("start"), environment.get("end")
    if not isinstance(before, dict) or not isinstance(after, dict):
        raise ValueError(f"{directory.name}: environment snapshots missing")
    stable = (
        "host",
        "kernel",
        "cpu",
        "storage",
        "governor",
        "toolchain",
        "profiler",
        "node_version",
    )
    if any(not before.get(key) or before[key] != after.get(key) for key in stable):
        raise ValueError(f"{directory.name}: environment drift")
    if before["node_version"] != NODE_VERSION:
        raise ValueError(f"{directory.name}: Node version mismatch")
    swap_delta = after.get("swap_pages", -1) - before.get("swap_pages", -1)
    if swap_delta < 0 or before.get("competing_jobs") or after.get("competing_jobs"):
        raise ValueError(f"{directory.name}: swap counter or competing job invalid")
    if (
        min(before.get("disk_free_bytes", 0), after.get("disk_free_bytes", 0))
        < 1_073_741_824
    ):
        raise ValueError(f"{directory.name}: insufficient disk space")
    if resources.get("method") != "gnu-time" or resources.get("swap_events") != 0:
        raise ValueError(f"{directory.name}: child resources missing or swapped")
    values = dict(
        line.split("=", 1)
        for line in (directory / "resource.txt").read_text().splitlines()
        if "=" in line
    )
    for key, resource_key in (
        ("user_s", "user_cpu_s"),
        ("system_s", "system_cpu_s"),
        ("peak_rss_kib", "peak_rss_kib"),
        ("fs_inputs", "fs_inputs"),
        ("fs_outputs", "fs_outputs"),
        ("major_faults", "major_faults"),
        ("swap_events", "swap_events"),
    ):
        observed = float(values[key]) if key.endswith("_s") else int(values[key])
        if resources.get(resource_key) != observed:
            raise ValueError(
                f"{directory.name}: resource summary differs from GNU Time"
            )
    warnings = []
    if swap_delta:
        warnings.append(f"host swap drift: {swap_delta} pages; child swap events: 0")
    if resources["major_faults"]:
        warnings.append(f"child major faults: {resources['major_faults']}")
    if environment.get("warnings") != warnings:
        raise ValueError(f"{directory.name}: environment warning mismatch")
    observation = audit_observations(raw, size, summary)
    observation.update(
        {
            "directory": directory.name,
            "raw_sha256": _hash(directory / "raw.json"),
            "runner_sha256": _hash(directory / "runner.mjs"),
            "protocol_sha256": _hash(directory / "pilot-protocol.json"),
            "host_swap_pages_delta": swap_delta,
            "child_resources": resources,
            "warnings": warnings,
        }
    )
    return observation


def audit_collection(root: Path, main: Path, platform: Path, checkout: Path) -> dict:
    """Audit five blocks per size and report baseline-only spread."""
    expected_dirs = {
        f"{size}-{number:02}-baseline" for size in (32, 256) for number in range(1, 6)
    }
    actual_dirs = {path.name for path in root.iterdir() if path.is_dir()}
    if actual_dirs != expected_dirs:
        raise ValueError("noise pilot block inventory differs from ten baseline blocks")
    archives = audit_archives(main, platform)
    source_identity = audit_source(checkout)
    blocks = [
        audit_block(root / f"{size}-{number:02}-baseline", size, source_identity)
        for size in (32, 256)
        for number in range(1, 6)
    ]
    runner_hashes = {block["runner_sha256"] for block in blocks}
    if len(runner_hashes) != 1:
        raise ValueError("workload runner changed between pilot blocks")
    starts = [datetime.fromisoformat(block["started_utc"]) for block in blocks]
    if starts != sorted(starts):
        raise ValueError("pilot run order differs from declared block order")
    spread = {}
    for size in (32, 256):
        selected = [
            block for block in blocks if block["directory"].startswith(f"{size}-")
        ]
        spread[str(size)] = {}
        for kind in KINDS:
            values = [block["cells"][kind]["warm_p95_ns"] for block in selected]
            median = statistics.median(values)
            spread[str(size)][kind] = {
                "p95_min_ns": min(values),
                "p95_median_ns": median,
                "p95_max_ns": max(values),
                "p95_range_pct_of_median": round(
                    100 * (max(values) - min(values)) / median, 3
                ),
            }
    return {
        "schema_version": 1,
        "scope": "baseline-only installed TypeScript S01 noise pilot; no candidate comparison",
        "source_sha": SOURCE_SHA,
        "source_identity": source_identity,
        "archives": archives,
        "runner_sha256": next(iter(runner_hashes)),
        "valid_blocks": len(blocks),
        "checked_attempts": sum(block["checked_attempts"] for block in blocks),
        "host_swap_warning_blocks": sum(
            bool(block["host_swap_pages_delta"]) for block in blocks
        ),
        "child_swap_events": sum(
            block["child_resources"]["swap_events"] for block in blocks
        ),
        "spread": spread,
        "blocks": blocks,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pilot-root", type=Path, required=True)
    parser.add_argument("--main-archive", type=Path, required=True)
    parser.add_argument("--platform-archive", type=Path, required=True)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit_collection(
        args.pilot_root.resolve(),
        args.main_archive.resolve(),
        args.platform_archive.resolve(),
        args.checkout.resolve(),
    )
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(
        f"VALID_TS_S01_BASELINE_NOISE: {result['valid_blocks']} blocks, "
        f"{result['checked_attempts']} call attempts"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
