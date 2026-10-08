#!/usr/bin/env python3
"""Recompute installed Python S01 paired results from retained raw blocks."""

from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
import math
from pathlib import Path
import statistics


KINDS = ("text", "vector", "hybrid")
QUERIES = {
    "text": "brightblue",
    "vector": "boat schedule across water",
    "hybrid": "harbor ferry",
}
MODEL = "fathomdb-bge-small-en-v1.5 default embedder"
WHEEL_NAME = "fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl"


def _read(path: Path) -> dict:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise ValueError(f"{path}: JSON root must be an object")
    return value


def _hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _rank(values: list[int], percentile: int) -> int:
    return sorted(values)[(len(values) * percentile + 99) // 100 - 1]


def _check_result(value: object, kind: str, eligible: set[str]) -> int:
    if not isinstance(value, dict) or value.get("semantic_ok") is not True:
        raise ValueError(f"{kind}: missing or failed semantic attempt")
    elapsed = value.get("latency_ns")
    ids = value.get("ids")
    branches = value.get("branches")
    if type(elapsed) is not int or elapsed <= 0:
        raise ValueError(f"{kind}: invalid call duration")
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
        or len(set(ids)) != len(ids)
    ):
        raise ValueError(f"{kind}: malformed or ineligible result")
    if kind == "text" and (ids != ["A"] or branches != ["text"]):
        raise ValueError("text anchor result changed")
    if kind == "vector" and ("vector" not in branches or "text" in branches):
        raise ValueError("vector-only branch changed")
    if kind == "hybrid" and "A" not in ids:
        raise ValueError("hybrid anchor missing")
    return elapsed


def audit_observations(
    raw: dict, *, size: int, samples: int, summary: dict | None = None
) -> dict:
    """Check every materialized call and independently calculate percentiles."""
    if raw.get("corpus_size") != size or raw.get("warm_samples_per_cell") != samples:
        raise ValueError("raw corpus size or warm sample count mismatch")
    if raw.get("queries") != QUERIES or raw.get("model") != MODEL:
        raise ValueError("raw query or model mismatch")
    cells = raw.get("cells")
    if not isinstance(cells, dict) or set(cells) != set(KINDS):
        raise ValueError("raw cell set mismatch")
    eligible = {"A", "B"} | {f"F{number:04}" for number in range(size - 2)}
    result: dict = {"cells": {}, "checked_attempts": 0}
    for kind in KINDS:
        cell = cells[kind]
        if not isinstance(cell, dict) or set(cell) != {
            "session_cold",
            "warmup",
            "warm",
        }:
            raise ValueError(f"{kind}: cell shape mismatch")
        warm = cell["warm"]
        if not isinstance(warm, list) or len(warm) != samples:
            raise ValueError(f"{kind}: warm sample count mismatch")
        first_ns = _check_result(cell["session_cold"], kind, eligible)
        _check_result(cell["warmup"], kind, eligible)
        durations = [_check_result(item, kind, eligible) for item in warm]
        values = {
            "session_cold_ns": first_ns,
            "warm_count": len(durations),
            "warm_p50_ns": _rank(durations, 50),
            "warm_p95_ns": _rank(durations, 95),
            "warm_p99_ns": _rank(durations, 99),
            "warm_max_ns": max(durations),
        }
        result["cells"][kind] = values
        result["checked_attempts"] += len(durations) + 2
    if raw.get("semantic_failures") != 0:
        raise ValueError("worker reported semantic failures")
    if summary is not None:
        if summary.get("checked_attempts") != result["checked_attempts"]:
            raise ValueError("summary checked-attempt count mismatch")
        reported = summary.get("cells")
        if not isinstance(reported, dict) or set(reported) != set(KINDS):
            raise ValueError("summary cell set mismatch")
        for kind, fields in result["cells"].items():
            if reported[kind].get("p99") != "measured":
                raise ValueError(f"{kind}: p99 was not reported as measured")
            for name, expected in fields.items():
                if reported[kind].get(name) != expected:
                    raise ValueError(f"{kind}: {name} summary differs from raw data")
    started = datetime.fromisoformat(raw["started_utc"])
    finished = datetime.fromisoformat(raw["finished_utc"])
    if started.tzinfo is None or finished <= started:
        raise ValueError("raw timestamps invalid")
    result["started_utc"] = raw["started_utc"]
    result["finished_utc"] = raw["finished_utc"]
    result["elapsed_block_s"] = round((finished - started).total_seconds(), 6)
    return result


def child_resource_warnings(resources: dict) -> list[str]:
    """Reject child swap while retaining measured major faults as noise evidence."""
    faults = resources.get("major_faults")
    if (
        resources.get("method") != "gnu-time"
        or resources.get("unsupported") != []
        or resources.get("swap_events") != 0
        or type(faults) is not int
        or faults < 0
    ):
        raise ValueError("measured child resource invalidator")
    return [f"child major faults: {faults}"] if faults else []


def warning_block_names(blocks: list[dict], prefix: str) -> list[str]:
    """Name only blocks carrying a warning of the requested type."""
    return [
        block["directory"]
        for block in blocks
        if any(warning.startswith(prefix) for warning in block["warnings"])
    ]


def audit_block(directory: Path, specification: dict, protocol_sha: str) -> dict:
    """Validate one source/artifact-bound block against archived bytes."""
    parts = directory.name.split("-")
    if len(parts) != 3:
        raise ValueError(f"malformed block name {directory.name}")
    size, pair, role = int(parts[0]), int(parts[1]), parts[2]
    artifact = specification[role]
    attempt = _read(directory / "attempt.json")
    block_protocol = _read(directory / "pilot-protocol.json")
    source = _read(directory / "source.json")
    environment = _read(directory / "environment.json")
    resources = _read(directory / "resources.json")
    raw = _read(directory / "raw.json")
    summary = _read(directory / "summary.json")
    if (
        attempt.get("status") != "VALID_S01_PAIRED_BLOCK"
        or attempt.get("invalidators") != []
    ):
        raise ValueError(f"{directory.name}: block not valid")
    if (
        attempt.get("role") != role
        or attempt.get("comparison_protocol_sha256") != protocol_sha
    ):
        raise ValueError(f"{directory.name}: attempt role or frozen protocol mismatch")
    if any(attempt.get(key) != artifact[key] for key in ("source_sha", "wheel_sha256")):
        raise ValueError(f"{directory.name}: artifact identity mismatch")
    if (
        block_protocol.get("status") != "FROZEN_S01_PYTHON_PAIRED"
        or block_protocol.get("role") != role
        or block_protocol.get("comparison_protocol_sha256") != protocol_sha
        or block_protocol.get("source_sha") != artifact["source_sha"]
        or block_protocol.get("wheel_sha256") != artifact["wheel_sha256"]
        or block_protocol.get("corpus_size") != size
        or block_protocol.get("corpus_sha256")
        != specification["corpus_sha256_by_size"][str(size)]
        or block_protocol.get("samples_per_cell")
        != specification["warm_samples_per_cell"]
        or block_protocol.get("model") != MODEL
        or block_protocol.get("queries") != QUERIES
        or block_protocol.get("timing_mode") != "unprofiled"
    ):
        raise ValueError(
            f"{directory.name}: block protocol differs from frozen specification"
        )
    if source.get("source_sha") != artifact["source_sha"]:
        raise ValueError(f"{directory.name}: source receipt mismatch")
    if (
        raw.get("schema_version") != 1
        or raw.get("source_sha") != artifact["source_sha"]
        or raw.get("corpus_sha256") != specification["corpus_sha256_by_size"][str(size)]
        or not isinstance(raw.get("artifact"), dict)
        or raw["artifact"].get("wheel_sha256") != artifact["wheel_sha256"]
    ):
        raise ValueError(f"{directory.name}: raw identity mismatch")
    for field, filename in (
        ("protocol_sha256", "pilot-protocol.json"),
        ("runner_sha256", "runner.py"),
        ("summary_sha256", "summary.json"),
    ):
        if attempt.get(field) != _hash(directory / filename):
            raise ValueError(f"{directory.name}: {field} differs from retained bytes")
    if attempt["runner_sha256"] != specification["workload_runner_sha256"]:
        raise ValueError(f"{directory.name}: wrong workload runner")
    if (
        summary.get("raw_sha256") != _hash(directory / "raw.json")
        or summary.get("resource_sha256") != _hash(directory / "resource.txt")
        or summary.get("runner_sha256") != attempt["runner_sha256"]
    ):
        raise ValueError(f"{directory.name}: summary input hashes mismatch")
    if environment.get("invalidators") != []:
        raise ValueError(f"{directory.name}: environment invalidator present")
    before, after = environment.get("start"), environment.get("end")
    if not isinstance(before, dict) or not isinstance(after, dict):
        raise ValueError(f"{directory.name}: environment snapshot missing")
    stable_fields = (
        "host",
        "kernel",
        "cpu",
        "storage",
        "governor",
        "toolchain",
        "profiler",
    )
    if any(
        not before.get(key) or before[key] != after.get(key) for key in stable_fields
    ):
        raise ValueError(f"{directory.name}: host identity changed")
    if before.get("competing_jobs") or after.get("competing_jobs"):
        raise ValueError(f"{directory.name}: competing job present")
    if (
        min(before.get("disk_free_bytes", 0), after.get("disk_free_bytes", 0))
        < 1_073_741_824
    ):
        raise ValueError(f"{directory.name}: insufficient disk headroom")
    swap_delta = after.get("swap_pages", -1) - before.get("swap_pages", -1)
    if swap_delta < 0:
        raise ValueError(f"{directory.name}: swap counter reversed")
    try:
        resource_warnings = child_resource_warnings(resources)
    except ValueError as error:
        raise ValueError(f"{directory.name}: {error}") from error
    for field in ("user_cpu_s", "system_cpu_s"):
        value = resources.get(field)
        if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
            raise ValueError(f"{directory.name}: invalid {field} resource")
    for field in ("peak_rss_kib", "fs_inputs", "fs_outputs"):
        value = resources.get(field)
        if type(value) is not int or value < 0:
            raise ValueError(f"{directory.name}: invalid {field} resource")
    expected_warnings = (
        [f"host swap drift: {swap_delta} pages; child swap events: 0"]
        if swap_delta
        else []
    )
    if environment.get("warnings") != expected_warnings + resource_warnings:
        raise ValueError(f"{directory.name}: environment warning differs from counters")
    report = audit_observations(
        raw, size=size, samples=specification["warm_samples_per_cell"], summary=summary
    )
    report.update(
        {
            "directory": directory.name,
            "size": size,
            "pair": pair,
            "role": role,
            "raw_sha256": _hash(directory / "raw.json"),
            "host": before["host"],
            "cpu": before["cpu"],
            "governor": before["governor"],
            "host_swap_pages_delta": swap_delta,
            "warnings": expected_warnings + resource_warnings,
            "child_resources": resources,
        }
    )
    return report


def _expected_order(specification: dict) -> list[tuple[int, int, str]]:
    sequence = []
    for size in specification["rows"]:
        for pair, roles in enumerate(specification["pair_order_each_size"], 1):
            for role in roles:
                sequence.append((size, pair, role))
    return sequence


def audit_collection(root: Path, protocol: Path) -> dict:
    """Verify all pairs and produce descriptive paired-delta summaries."""
    specification = _read(protocol)
    if specification.get("status") != "FROZEN_S01_PYTHON_PAIRED":
        raise ValueError("comparison protocol is not frozen")
    protocol_sha = _hash(protocol)
    if specification.get("warm_samples_per_cell") != 1000:
        raise ValueError("comparison does not meet the p99 sample floor")
    if _hash(root / "block-runner.py") != specification.get("block_runner_sha256"):
        raise ValueError("archived block runner differs from frozen protocol")
    excluded = root / "invalid-pre-p99-fix"
    excluded_attempt = _read(excluded / "attempt.json")
    excluded_summary = _read(excluded / "summary.json")
    if (
        excluded_attempt.get("comparison_protocol_sha256")
        != specification.get("superseded_protocol_sha256")
        or excluded_summary["cells"]["text"].get("p99")
        != "unsupported_below_1000_samples"
    ):
        raise ValueError("excluded first attempt does not demonstrate the p99 defect")
    expected = _expected_order(specification)
    actual_directories = {path.name for path in root.iterdir() if path.is_dir()}
    expected_directories = {f"{size}-{pair:02}-{role}" for size, pair, role in expected}
    if actual_directories != expected_directories | {"invalid-pre-p99-fix"}:
        raise ValueError(
            "paired block inventory incomplete or contains unexpected directories"
        )
    wheel_dirs = {
        "baseline": "2026-10-07-python-wheel-baseline",
        "candidate": "2026-10-07-python-wheel-qualification",
    }
    for role, dirname in wheel_dirs.items():
        if (
            _hash(root.parent / dirname / WHEEL_NAME)
            != specification[role]["wheel_sha256"]
        ):
            raise ValueError(
                f"{role}: archived installed wheel differs from frozen protocol"
            )
    blocks = [
        audit_block(root / f"{size}-{pair:02}-{role}", specification, protocol_sha)
        for size, pair, role in expected
    ]
    if len({block["raw_sha256"] for block in blocks}) != len(blocks):
        raise ValueError("duplicate raw block bytes")
    host_identity = {
        (block["host"], block["cpu"], block["governor"]) for block in blocks
    }
    if len(host_identity) != 1:
        raise ValueError("host identity changed between paired blocks")
    for previous, current in zip(blocks, blocks[1:]):
        pause = (
            datetime.fromisoformat(current["started_utc"])
            - datetime.fromisoformat(previous["finished_utc"])
        ).total_seconds()
        if pause < specification["minimum_idle_seconds_between_blocks"]:
            raise ValueError("paired block order or minimum idle interval violated")
    driver_records = [
        json.loads(line) for line in (root / "run-order.jsonl").read_text().splitlines()
    ]
    if len(driver_records) != len(expected) - 2:
        raise ValueError("campaign driver run-order length mismatch")
    for recorded, (size, pair, role) in zip(driver_records, expected[2:]):
        if (
            (recorded.get("size"), recorded.get("pair"), recorded.get("role"))
            != (size, pair, role)
            or recorded.get("exit_code") != 0
            or recorded.get("attempt_status") != "VALID_S01_PAIRED_BLOCK"
            or recorded.get("invalidators") != []
            or recorded.get("protocol_sha256") != protocol_sha
            or Path(recorded.get("output_dir", "")).name
            != f"slice135-s01-paired-v2-{size}-{pair:02}-{role}"
        ):
            raise ValueError(
                "campaign driver run-order record differs from frozen schedule"
            )
    report: dict = {
        "schema_version": 1,
        "scope": "installed Python S01 paired diagnostic; not the full Phase 1 checkpoint",
        "protocol_sha256": protocol_sha,
        "block_runner_sha256": _hash(root / "block-runner.py"),
        "campaign_driver_sha256": _hash(root / "remaining-driver.py"),
        "valid_blocks": len(blocks),
        "checked_attempts": sum(block["checked_attempts"] for block in blocks),
        "excluded_attempts": ["invalid-pre-p99-fix"],
        "host_swap_warning_blocks": [
            block["directory"] for block in blocks if block["warnings"]
        ],
        "sizes": {},
    }
    for size in specification["rows"]:
        size_blocks = [block for block in blocks if block["size"] == size]
        paired = {
            pair: {
                block["role"]: block for block in size_blocks if block["pair"] == pair
            }
            for pair in range(1, specification["pairs_per_size"] + 1)
        }
        cell_summaries = {}
        for kind in KINDS:
            by_percentile = {}
            for percentile in (50, 95, 99):
                field = f"warm_p{percentile}_ns"
                deltas = []
                clean_deltas = []
                for pair in paired.values():
                    baseline, candidate = pair["baseline"], pair["candidate"]
                    b = baseline["cells"][kind][field]
                    c = candidate["cells"][kind][field]
                    delta = 100 * (c - b) / b
                    deltas.append(delta)
                    if not baseline["warnings"] and not candidate["warnings"]:
                        clean_deltas.append(delta)
                by_percentile[f"p{percentile}"] = {
                    "pair_deltas_pct": [round(delta, 4) for delta in deltas],
                    "median_delta_pct": round(statistics.median(deltas), 4),
                    "observed_range_pct": [
                        round(min(deltas), 4),
                        round(max(deltas), 4),
                    ],
                    "clean_pair_deltas_pct": [
                        round(delta, 4) for delta in clean_deltas
                    ],
                    "clean_pair_count": len(clean_deltas),
                    "clean_pair_inference": "insufficient"
                    if len(clean_deltas) < 3
                    else "descriptive_only",
                }
            cell_summaries[kind] = by_percentile
        report["sizes"][str(size)] = {
            "pairs": len(paired),
            "checked_attempts": sum(block["checked_attempts"] for block in size_blocks),
            "cells": cell_summaries,
            "block_receipts": size_blocks,
        }
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("protocol", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    serialized = (
        json.dumps(audit_collection(args.root, args.protocol), indent=2, sort_keys=True)
        + "\n"
    )
    if args.output:
        args.output.write_text(serialized)
    else:
        print(serialized, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
