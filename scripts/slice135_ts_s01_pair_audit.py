#!/usr/bin/env python3
"""Independently recompute frozen installed-TypeScript S01 paired receipts."""

from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess
import tarfile


KINDS = ("text", "vector", "hybrid")
NATIVE_SHA256 = {
    "baseline": "4a3aef2833060cde858faad689e4c734de31dc70e9dab13a00a6594f694c60ca",
    "candidate": "cb6402660e90f1dcc7d21fbecda7d44c50305ec62c355d6b71a5fd4aa4606a9a",
}
MODULE_SHA256 = {
    "baseline": "9d86c756e481ee09581ff8c687fd74de8cefbf2ad825a582015cb7b703ea4dc7",
    "candidate": "cc0ea0b2ae641aa5162a223985d9afb5ba1233b073899e66a2847f92f36bf432",
}
ARCHIVES = {
    "baseline": ("fathomdb-0.8.26.tgz", "fathomdb-linux-x64-gnu-0.8.26.tgz"),
    "candidate": (
        "candidate-fathomdb-0.8.26.tgz",
        "candidate-fathomdb-linux-x64-gnu-0.8.26.tgz",
    ),
}


def _read(path: Path) -> dict:
    result = json.loads(path.read_text())
    if not isinstance(result, dict):
        raise ValueError(f"{path}: JSON root is not an object")
    return result


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
    raw: dict,
    *,
    role: str,
    size: int,
    specification: dict,
    summary: dict | None = None,
) -> dict:
    """Check every materialized result and recompute nearest-rank tails."""
    if (
        raw.get("schema_version") != 1
        or raw.get("source_sha") != specification[role]["source_sha"]
    ):
        raise ValueError("raw source identity mismatch")
    artifact = raw.get("artifact")
    if (
        not isinstance(artifact, dict)
        or artifact.get("native_sha256") != NATIVE_SHA256[role]
    ):
        raise ValueError("raw native artifact identity mismatch")
    if (
        artifact.get("module_sha256") != MODULE_SHA256[role]
        or artifact.get("node_version") != specification["node_version"]
    ):
        raise ValueError("raw installed module or Node identity mismatch")
    if (
        raw.get("corpus_size") != size
        or raw.get("corpus_sha256") != specification["corpus_sha256_by_size"][str(size)]
        or raw.get("warm_samples_per_cell") != specification["warm_samples_per_cell"]
    ):
        raise ValueError("raw corpus or sample identity mismatch")
    if (
        raw.get("model") != specification["model"]
        or raw.get("queries") != specification["queries"]
    ):
        raise ValueError("raw model or query mismatch")
    cells = raw.get("cells")
    if not isinstance(cells, dict) or set(cells) != set(KINDS):
        raise ValueError("raw cell set mismatch")
    eligible = {"A", "B"} | {f"F{number:04}" for number in range(size - 2)}
    samples = specification["warm_samples_per_cell"]
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


def audit_archives(archives_root: Path, specification: dict) -> dict:
    """Check archived npm bytes and the measured JS/native members."""
    report = {}
    for role, names in ARCHIVES.items():
        main, platform = (archives_root / name for name in names)
        if (
            _hash(main) != specification[role]["main_archive_sha256"]
            or _hash(platform) != specification[role]["platform_archive_sha256"]
        ):
            raise ValueError(f"{role}: archive hash differs from frozen protocol")
        for archive, member_name, expected in (
            (main, "package/dist/index.js", MODULE_SHA256[role]),
            (platform, "package/fathomdb.linux-x64-gnu.node", NATIVE_SHA256[role]),
        ):
            with tarfile.open(archive, mode="r:gz") as package:
                member = package.extractfile(member_name)
                if (
                    member is None
                    or hashlib.sha256(member.read()).hexdigest() != expected
                ):
                    raise ValueError(
                        f"{role}: archive member differs from measured bytes"
                    )
        report[role] = {
            "main_archive_sha256": specification[role]["main_archive_sha256"],
            "platform_archive_sha256": specification[role]["platform_archive_sha256"],
            "module_sha256": MODULE_SHA256[role],
            "native_sha256": NATIVE_SHA256[role],
        }
    return report


def audit_source(checkout: Path, source_sha: str) -> dict:
    """Check source HEAD, cleanliness and package/dependency lock bytes."""
    for arguments, expected in (
        (["rev-parse", "HEAD"], source_sha),
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
            raise ValueError("measured source checkout differs from frozen identity")
    return {
        "source_sha": source_sha,
        "cargo_lock_sha256": _hash(checkout / "Cargo.lock"),
        "npm_lock_sha256": _hash(checkout / "src/ts/package-lock.json"),
    }


def audit_block(
    directory: Path,
    specification: dict,
    protocol_sha: str,
    source_identity: dict,
    archive_identity: dict,
) -> dict:
    """Validate one independently materialized paired block."""
    parts = directory.name.split("-")
    if len(parts) != 3:
        raise ValueError(f"malformed block name: {directory.name}")
    size, pair, role = int(parts[0]), int(parts[1]), parts[2]
    attempt = _read(directory / "attempt.json")
    block_protocol = _read(directory / "pilot-protocol.json")
    source = _read(directory / "source.json")
    artifacts = _read(directory / "artifacts.json")
    environment = _read(directory / "environment.json")
    resources = _read(directory / "resources.json")
    raw = _read(directory / "raw.json")
    summary = _read(directory / "summary.json")
    command = json.loads((directory / "run-command.json").read_text())
    if not isinstance(command, list) or any(type(part) is not str for part in command):
        raise ValueError(f"{directory.name}: command record malformed")
    for option, expected in (
        ("--rows", str(size)),
        ("--samples", str(specification["warm_samples_per_cell"])),
        ("--source-sha", specification[role]["source_sha"]),
        ("--expected-native-sha256", NATIVE_SHA256[role]),
    ):
        if option not in command or command[command.index(option) + 1] != expected:
            raise ValueError(
                f"{directory.name}: command differs from protocol: {option}"
            )
    if (
        attempt.get("status") != "VALID_TS_S01_PAIRED_BLOCK"
        or attempt.get("invalidators") != []
        or attempt.get("role") != role
        or attempt.get("comparison_protocol_sha256") != protocol_sha
        or attempt.get("source_sha") != specification[role]["source_sha"]
    ):
        raise ValueError(f"{directory.name}: block status or identity invalid")
    if source != source_identity or artifacts != archive_identity:
        raise ValueError(f"{directory.name}: source or package bytes differ")
    for field, filename in (
        ("protocol_sha256", "pilot-protocol.json"),
        ("runner_sha256", "runner.mjs"),
        ("summary_sha256", "summary.json"),
    ):
        if attempt.get(field) != _hash(directory / filename):
            raise ValueError(f"{directory.name}: {field} differs from retained bytes")
    if attempt["runner_sha256"] != specification["workload_runner_sha256"]:
        raise ValueError(f"{directory.name}: workload runner changed")
    if (
        block_protocol.get("status") != "FROZEN_TS_S01_PAIRED"
        or block_protocol.get("role") != role
        or block_protocol.get("comparison_protocol_sha256") != protocol_sha
        or block_protocol.get("source_sha") != specification[role]["source_sha"]
        or block_protocol.get("corpus_size") != size
        or block_protocol.get("corpus_sha256")
        != specification["corpus_sha256_by_size"][str(size)]
        or block_protocol.get("warm_samples_per_cell")
        != specification["warm_samples_per_cell"]
        or block_protocol.get("model") != specification["model"]
        or block_protocol.get("queries") != specification["queries"]
        or block_protocol.get("node_version") != specification["node_version"]
    ):
        raise ValueError(f"{directory.name}: block protocol differs from freeze")
    if any(block_protocol.get(key) != value for key, value in archive_identity.items()):
        raise ValueError(f"{directory.name}: block archive identity differs")
    if (
        summary.get("raw_sha256") != _hash(directory / "raw.json")
        or summary.get("resource_sha256") != _hash(directory / "resource.txt")
        or summary.get("runner_sha256") != attempt["runner_sha256"]
    ):
        raise ValueError(f"{directory.name}: summary input hashes differ")
    if (
        raw.get("artifact", {}).get("module_sha256")
        != archive_identity["module_sha256"]
        or raw.get("artifact", {}).get("native_sha256")
        != archive_identity["native_sha256"]
    ):
        raise ValueError(f"{directory.name}: raw artifact differs from archive")
    if environment.get("invalidators") != []:
        raise ValueError(f"{directory.name}: environment invalidator")
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
        "node_version",
    )
    if any(
        not before.get(key) or before[key] != after.get(key) for key in stable_fields
    ):
        raise ValueError(f"{directory.name}: host identity changed")
    if before["node_version"] != specification["node_version"]:
        raise ValueError(f"{directory.name}: Node version mismatch")
    if before.get("competing_jobs") or after.get("competing_jobs"):
        raise ValueError(f"{directory.name}: competing job present")
    if (
        min(before.get("disk_free_bytes", 0), after.get("disk_free_bytes", 0))
        < 1_073_741_824
    ):
        raise ValueError(f"{directory.name}: insufficient disk headroom")
    swap_delta = after.get("swap_pages", -1) - before.get("swap_pages", -1)
    if swap_delta < 0:
        raise ValueError(f"{directory.name}: host swap counter reversed")
    if resources.get("method") != "gnu-time" or resources.get("unsupported") != []:
        raise ValueError(f"{directory.name}: child resource report missing")
    if resources.get("swap_events") != 0:
        raise ValueError(f"{directory.name}: measured child swapped")
    for field in ("user_cpu_s", "system_cpu_s"):
        value = resources.get(field)
        if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
            raise ValueError(f"{directory.name}: {field} invalid")
    for field in ("peak_rss_kib", "fs_inputs", "fs_outputs", "major_faults"):
        if type(resources.get(field)) is not int or resources[field] < 0:
            raise ValueError(f"{directory.name}: {field} invalid")
    values = dict(
        line.split("=", 1)
        for line in (directory / "resource.txt").read_text().splitlines()
        if "=" in line
    )
    for key, field in (
        ("user_s", "user_cpu_s"),
        ("system_s", "system_cpu_s"),
        ("peak_rss_kib", "peak_rss_kib"),
        ("fs_inputs", "fs_inputs"),
        ("fs_outputs", "fs_outputs"),
        ("major_faults", "major_faults"),
        ("swap_events", "swap_events"),
    ):
        observed = float(values[key]) if key.endswith("_s") else int(values[key])
        if resources.get(field) != observed:
            raise ValueError(
                f"{directory.name}: resource summary differs from GNU Time"
            )
    warnings = []
    if swap_delta:
        warnings.append(f"host swap drift: {swap_delta} pages; child swap events: 0")
    if resources["major_faults"]:
        warnings.append(f"child major faults: {resources['major_faults']}")
    if environment.get("warnings") != warnings:
        raise ValueError(f"{directory.name}: host warning differs from counters")
    report = audit_observations(
        raw, role=role, size=size, specification=specification, summary=summary
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
            "warnings": warnings,
            "child_resources": resources,
        }
    )
    return report


def _expected_order(specification: dict) -> list[tuple[int, int, str]]:
    return [
        (size, pair, role)
        for size in specification["rows"]
        for pair, roles in enumerate(specification["pair_order_each_size"], 1)
        for role in roles
    ]


def audit_collection(
    root: Path,
    protocol_path: Path,
    archives_root: Path,
    baseline_checkout: Path,
    candidate_checkout: Path,
) -> dict:
    """Verify frozen order and return descriptive paired-delta summaries."""
    specification = _read(protocol_path)
    if specification.get("status") != "FROZEN_TS_S01_PAIRED":
        raise ValueError("TypeScript comparison protocol is not frozen")
    if specification.get("warm_samples_per_cell") != 1000:
        raise ValueError("TypeScript comparison does not meet the p99 sample floor")
    protocol_sha = _hash(protocol_path)
    if _hash(root / "block-runner.py") != specification.get("block_runner_sha256"):
        raise ValueError("archived block runner differs from frozen protocol")
    if _hash(root / "workload-runner.mjs") != specification.get(
        "workload_runner_sha256"
    ):
        raise ValueError("archived workload runner differs from frozen protocol")
    archives = audit_archives(archives_root, specification)
    sources = {
        "baseline": audit_source(
            baseline_checkout, specification["baseline"]["source_sha"]
        ),
        "candidate": audit_source(
            candidate_checkout, specification["candidate"]["source_sha"]
        ),
    }
    expected = _expected_order(specification)
    expected_directories = {f"{size}-{pair:02}-{role}" for size, pair, role in expected}
    actual_directories = {path.name for path in root.iterdir() if path.is_dir()}
    if actual_directories != expected_directories:
        raise ValueError("paired block inventory incomplete or unexpected")
    blocks = [
        audit_block(
            root / f"{size}-{pair:02}-{role}",
            specification,
            protocol_sha,
            sources[role],
            archives[role],
        )
        for size, pair, role in expected
    ]
    if len({block["raw_sha256"] for block in blocks}) != len(blocks):
        raise ValueError("duplicate raw paired block bytes")
    if len({(block["host"], block["cpu"], block["governor"]) for block in blocks}) != 1:
        raise ValueError("paired host identity changed between blocks")
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
    if len(driver_records) != len(expected) * 2:
        raise ValueError("campaign driver run-order length mismatch")
    for index, (size, pair, role) in enumerate(expected):
        start, finish = driver_records[2 * index : 2 * index + 2]
        block_name = f"{size}-{pair:02}-{role}"
        if (
            start.get("event") != "start"
            or finish.get("event") != "finish"
            or start.get("block") != block_name
            or finish.get("block") != block_name
            or start.get("index") != index
            or finish.get("index") != index
            or finish.get("returncode") != 0
            or start.get("monotonic_ns", -1) >= finish.get("monotonic_ns", -1)
        ):
            raise ValueError(
                "campaign driver order record differs from frozen schedule"
            )
        if (
            index
            and start["monotonic_ns"] - driver_records[2 * index - 1]["monotonic_ns"]
            < specification["minimum_idle_seconds_between_blocks"] * 1_000_000_000
        ):
            raise ValueError("campaign driver idle interval too short")
    report: dict = {
        "schema_version": 1,
        "scope": "installed TypeScript S01 paired diagnostic; not the full Phase 1 checkpoint",
        "protocol_sha256": protocol_sha,
        "block_runner_sha256": _hash(root / "block-runner.py"),
        "workload_runner_sha256": _hash(root / "workload-runner.mjs"),
        "campaign_driver_sha256": _hash(root / "campaign-driver.py"),
        "archives": archives,
        "sources": sources,
        "valid_blocks": len(blocks),
        "checked_attempts": sum(block["checked_attempts"] for block in blocks),
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
                deltas, clean_deltas = [], []
                for pair in paired.values():
                    baseline, candidate = pair["baseline"], pair["candidate"]
                    delta = (
                        100
                        * (
                            candidate["cells"][kind][field]
                            - baseline["cells"][kind][field]
                        )
                        / baseline["cells"][kind][field]
                    )
                    deltas.append(delta)
                    if not baseline["warnings"] and not candidate["warnings"]:
                        clean_deltas.append(delta)
                by_percentile[f"p{percentile}"] = {
                    "pair_deltas_pct": [round(value, 4) for value in deltas],
                    "median_delta_pct": round(statistics.median(deltas), 4),
                    "observed_range_pct": [
                        round(min(deltas), 4),
                        round(max(deltas), 4),
                    ],
                    "clean_pair_deltas_pct": [
                        round(value, 4) for value in clean_deltas
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
    parser.add_argument("--paired-root", type=Path, required=True)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--archives-root", type=Path, required=True)
    parser.add_argument("--baseline-checkout", type=Path, required=True)
    parser.add_argument("--candidate-checkout", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = audit_collection(
        args.paired_root.resolve(),
        args.protocol.resolve(),
        args.archives_root.resolve(),
        args.baseline_checkout.resolve(),
        args.candidate_checkout.resolve(),
    )
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(
        f"VALID_TS_S01_PAIRED: {report['valid_blocks']} blocks, "
        f"{report['checked_attempts']} call attempts"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
