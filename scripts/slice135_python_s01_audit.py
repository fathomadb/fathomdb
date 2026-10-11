#!/usr/bin/env python3
"""Independently audit retained baseline-only Python S01 pilot receipts."""

from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import statistics


SOURCE_SHA = "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
WHEEL_SHA256 = "7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282"
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


def _read(path: Path) -> dict:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise ValueError(f"{path}: JSON root is not an object")
    return value


def _hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _rank(values: list[int], numerator: int) -> int:
    index = (len(values) * numerator + 99) // 100 - 1
    return sorted(values)[index]


def _attempt(value: object, kind: str, eligible: set[str]) -> int:
    if not isinstance(value, dict) or value.get("semantic_ok") is not True:
        raise ValueError(f"{kind}: malformed or failed attempt")
    elapsed = value.get("latency_ns")
    if type(elapsed) is not int or elapsed <= 0:
        raise ValueError(f"{kind}: invalid latency")
    ids = value.get("ids")
    branches = value.get("branches")
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
    """Recompute result validity and supported percentiles without the runner."""
    if raw.get("schema_version") != 1 or raw.get("source_sha") != SOURCE_SHA:
        raise ValueError("raw source identity mismatch")
    artifact = raw.get("artifact")
    if not isinstance(artifact, dict) or artifact.get("wheel_sha256") != WHEEL_SHA256:
        raise ValueError("raw wheel identity mismatch")
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
            "p99": "unsupported",
        }
        result["checked_attempts"] += 2 + len(durations)
    if raw.get("semantic_failures") != result["semantic_failures"]:
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
                declared = reported[kind].get(field)
                if field == "p99":
                    if declared != "unsupported_below_1000_samples":
                        raise ValueError(
                            f"{kind}: unsupported tail was reported as measured"
                        )
                    continue
                if field.startswith("warm_p") and declared != expected:
                    raise ValueError(f"{kind}: percentile mismatch for {field}")
                if declared != expected and not field.startswith("warm_p"):
                    raise ValueError(f"{kind}: summary mismatch for {field}")
    return result


def audit_block(directory: Path, size: int) -> dict:
    """Verify a block's raw bytes, identities, environment and recomputation."""
    attempt = _read(directory / "attempt.json")
    protocol = _read(directory / "pilot-protocol.json")
    source = _read(directory / "source.json")
    environment = _read(directory / "environment.json")
    resources = _read(directory / "resources.json")
    raw = _read(directory / "raw.json")
    summary = _read(directory / "summary.json")
    expected = {
        "source_sha": SOURCE_SHA,
        "wheel_sha256": WHEEL_SHA256,
        "corpus_size": size,
        "corpus_sha256": CORPUS_SHA256[size],
        "model": MODEL,
        "queries": QUERIES,
        "samples_per_cell": SAMPLES,
        "timing_mode": "unprofiled",
    }
    if any(protocol.get(key) != value for key, value in expected.items()):
        raise ValueError(
            f"{directory.name}: pilot protocol differs from expected fixture"
        )
    if protocol.get("schema_version") != 1 or source.get("source_sha") != SOURCE_SHA:
        raise ValueError(f"{directory.name}: protocol or source receipt mismatch")
    if (
        attempt.get("status") != "VALID_BASELINE_PILOT_BLOCK"
        or attempt.get("invalidators") != []
    ):
        raise ValueError(f"{directory.name}: block was not valid")
    for field, path in (
        ("protocol_sha256", "pilot-protocol.json"),
        ("runner_sha256", "runner.py"),
        ("summary_sha256", "summary.json"),
    ):
        if attempt.get(field) != _hash(directory / path):
            raise ValueError(f"{directory.name}: {field} differs from bytes")
    if protocol.get("runner_sha256") != _hash(directory / "runner.py"):
        raise ValueError(f"{directory.name}: runner differs from protocol")
    if summary.get("raw_sha256") != _hash(directory / "raw.json"):
        raise ValueError(f"{directory.name}: raw differs from summary hash")
    if summary.get("resource_sha256") != _hash(directory / "resource.txt"):
        raise ValueError(f"{directory.name}: resource differs from summary hash")
    if summary.get("runner_sha256") != _hash(directory / "runner.py"):
        raise ValueError(f"{directory.name}: runner differs from summary hash")
    if any(
        attempt.get(field) != value
        for field, value in (("source_sha", SOURCE_SHA), ("wheel_sha256", WHEEL_SHA256))
    ):
        raise ValueError(f"{directory.name}: attempt identity mismatch")
    if environment.get("invalidators") != []:
        raise ValueError(f"{directory.name}: environment invalidators")
    before = environment.get("start")
    after = environment.get("end")
    if not isinstance(before, dict) or not isinstance(after, dict):
        raise ValueError(f"{directory.name}: environment snapshots missing")
    stable = ("host", "kernel", "cpu", "storage", "governor", "profiler")
    if any(not before.get(key) or before[key] != after.get(key) for key in stable):
        raise ValueError(f"{directory.name}: environment drift")
    host_swap_delta = after.get("swap_pages", -1) - before.get("swap_pages", -1)
    if host_swap_delta < 0:
        raise ValueError(f"{directory.name}: host swap counter reversed")
    if before.get("competing_jobs") or after.get("competing_jobs"):
        raise ValueError(f"{directory.name}: competing job present")
    if (
        min(before.get("disk_free_bytes", 0), after.get("disk_free_bytes", 0))
        < 1_073_741_824
    ):
        raise ValueError(f"{directory.name}: insufficient disk space")
    if resources.get("method") != "gnu-time" or resources.get("swap_events") != 0:
        raise ValueError(f"{directory.name}: child resources missing or swapped")
    expected_warnings = (
        [f"host swap drift: {host_swap_delta} pages; child swap events: 0"]
        if host_swap_delta
        else []
    )
    if environment.get("warnings") != expected_warnings:
        raise ValueError(f"{directory.name}: warning record differs from host counters")
    result = audit_observations(raw, size, summary)
    result["directory"] = directory.name
    result["raw_sha256"] = _hash(directory / "raw.json")
    result["runner_sha256"] = _hash(directory / "runner.py")
    result["protocol_sha256"] = _hash(directory / "pilot-protocol.json")
    result["host"] = before["host"]
    result["cpu"] = before["cpu"]
    result["governor"] = before["governor"]
    result["child_resources"] = resources
    result["host_swap_pages_delta"] = host_swap_delta
    result["warnings"] = expected_warnings
    return result


def audit_collection(root: Path) -> dict:
    """Audit five blocks per size and rank observed between-block spread."""
    expected_dirs = {
        f"{size}-{number:02}" for size in (32, 256) for number in range(1, 6)
    }
    actual_dirs = {path.name for path in root.iterdir() if path.is_dir()}
    if actual_dirs != expected_dirs | {"invalid-venv-symlink"}:
        raise ValueError(
            "pilot block inventory differs from ten valid and one invalid attempt"
        )
    invalid = _read(root / "invalid-venv-symlink/attempt.json")
    if invalid.get("status") != "INVALID_BLOCK" or not invalid.get("invalidators"):
        raise ValueError("invalid launcher attempt was not retained")
    if (root / "invalid-venv-symlink/raw.json").exists():
        raise ValueError("invalid launcher attempt unexpectedly has raw measurements")
    report: dict = {
        "schema_version": 1,
        "scope": "baseline-only installed Python S01 noise pilot; no candidate comparison",
        "valid_blocks": 10,
        "invalid_retained": ["invalid-venv-symlink"],
        "sizes": {},
    }
    all_blocks = []
    for size in (32, 256):
        blocks = [
            audit_block(root / f"{size}-{number:02}", size) for number in range(1, 6)
        ]
        starts = [datetime.fromisoformat(block["started_utc"]) for block in blocks]
        if starts != sorted(starts):
            raise ValueError(f"{size}: block order differs from timestamps")
        if len({block["raw_sha256"] for block in blocks}) != 5:
            raise ValueError(f"{size}: repeated raw block bytes")
        if len({block["protocol_sha256"] for block in blocks}) != 1:
            raise ValueError(f"{size}: protocol changed during baseline pilot")
        cells = {}
        for kind in KINDS:
            p50 = [block["cells"][kind]["warm_p50_ns"] for block in blocks]
            p95 = [block["cells"][kind]["warm_p95_ns"] for block in blocks]
            cells[kind] = {
                "p50_ns_by_block": p50,
                "p95_ns_by_block": p95,
                "p50_spread_pct_of_median": round(
                    100 * (max(p50) - min(p50)) / statistics.median(p50), 4
                ),
                "p95_spread_pct_of_median": round(
                    100 * (max(p95) - min(p95)) / statistics.median(p95), 4
                ),
                "p99": "unsupported",
            }
        report["sizes"][str(size)] = {
            "blocks": len(blocks),
            "total_checked_attempts": sum(
                block["checked_attempts"] for block in blocks
            ),
            "cells": cells,
            "block_receipts": blocks,
        }
        all_blocks.extend(blocks)
    if (
        len(
            {
                (block["runner_sha256"], block["host"], block["cpu"], block["governor"])
                for block in all_blocks
            }
        )
        != 1
    ):
        raise ValueError("runner or host identity changed between corpus sizes")
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = audit_collection(args.root)
    serialized = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(serialized)
    else:
        print(serialized, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
