#!/usr/bin/env python3
"""Recompute S03 baseline noise, order, resources and persisted state."""

from __future__ import annotations

import argparse
from datetime import datetime
import json
from pathlib import Path
import statistics
from typing import Any

import slice135_python_s03_audit as sample_audit


ROOT = Path(__file__).resolve().parents[1]
SOURCE = "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
WHEEL_SHA256 = "7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282"


def expected_blocks() -> list[tuple[int, int, str]]:
    """Own the expected order independently of the campaign producer."""
    return [
        (size, index, f"{size}-block-{index:02}")
        for size in (32, 256)
        for index in range(1, 6)
    ]


def check_order(records: list[dict[str, Any]]) -> None:
    """Reject omitted, duplicated, reordered or under-idled blocks."""
    if len(records) != 10:
        raise ValueError("baseline block order must contain ten records")
    previous_finished = None
    for ordinal, (record, (size, index, name)) in enumerate(
        zip(records, expected_blocks()), 1
    ):
        if (
            record.get("ordinal"),
            record.get("size"),
            record.get("block_index"),
            record.get("name"),
        ) != (ordinal, size, index, name):
            raise ValueError("baseline block order differs from declared schedule")
        if "started_utc" not in record or "finished_utc" not in record:
            continue
        started = datetime.fromisoformat(record["started_utc"])
        finished = datetime.fromisoformat(record["finished_utc"])
        if finished <= started:
            raise ValueError(f"{name}: nonpositive block interval")
        if (
            previous_finished is not None
            and (started - previous_finished).total_seconds() < 19.9
        ):
            raise ValueError(f"{name}: baseline block idle interval below 20 seconds")
        previous_finished = finished


def _resources(directory: Path) -> dict[str, int | float]:
    """Reparse GNU Time separately from the block's resource JSON."""
    values = dict(
        line.split("=", 1)
        for line in (directory / "resource.txt").read_text().splitlines()
        if "=" in line
    )
    parsed: dict[str, int | float] = {
        "user_cpu_s": float(values["user_s"]),
        "system_cpu_s": float(values["system_s"]),
        "peak_rss_kib": int(values["peak_rss_kib"]),
        "fs_inputs": int(values["fs_inputs"]),
        "fs_outputs": int(values["fs_outputs"]),
        "major_faults": int(values["major_faults"]),
        "swap_events": int(values["swap_events"]),
    }
    retained = json.loads((directory / "resources.json").read_text())
    if any(retained.get(key) != value for key, value in parsed.items()):
        raise ValueError(f"{directory.name}: child resources differ from GNU Time")
    if parsed["swap_events"] != 0:
        raise ValueError(f"{directory.name}: measured child swapped")
    return parsed


def _environment(directory: Path) -> list[str]:
    report = json.loads((directory / "environment.json").read_text())
    if report.get("invalidators") != []:
        raise ValueError(f"{directory.name}: environment invalidators remain")
    start, end = report["start"], report["end"]
    for key in (
        "host",
        "kernel",
        "cpu",
        "storage",
        "governor",
        "toolchain",
        "profiler",
    ):
        if not start.get(key) or start[key] != end.get(key):
            raise ValueError(f"{directory.name}: environment identity drift")
    if start["competing_jobs"] or end["competing_jobs"]:
        raise ValueError(f"{directory.name}: competing jobs present")
    if min(start["disk_free_bytes"], end["disk_free_bytes"]) < 1_073_741_824:
        raise ValueError(f"{directory.name}: disk below pilot floor")
    if end["swap_pages"] < start["swap_pages"]:
        raise ValueError(f"{directory.name}: host swap counter decreased")
    return report["warnings"]


def audit(directory: Path) -> dict[str, Any]:
    """Open every retained block and independently recompute pilot spread."""
    manifest = json.loads((directory / "pilot-manifest.json").read_text())
    if (
        manifest.get("status") != "S03_BASELINE_NOISE_PILOT_NOT_FROZEN_COMPARISON"
        or manifest.get("baseline_source_sha") != SOURCE
    ):
        raise ValueError("baseline pilot manifest identity mismatch")
    if (
        manifest.get("wheel_sha256") != WHEEL_SHA256
        or manifest.get("repetitions_per_cell") != 100
        or manifest.get("idle_seconds", 0) < 20
    ):
        raise ValueError("baseline pilot wheel, count or idle interval changed")
    expected_files = {
        "block_runner_sha256": ROOT / "scripts/slice135_python_s03_block.py",
        "auditor_sha256": ROOT / "scripts/slice135_python_s03_audit.py",
        "workload_runner_sha256": ROOT / "scripts/slice135_python_s03.py",
        "campaign_runner_sha256": ROOT
        / "scripts/slice135_python_s03_baseline_campaign.py",
    }
    if any(
        manifest.get(key) != sample_audit.digest(path)
        for key, path in expected_files.items()
    ):
        raise ValueError("campaign executable bytes differ from manifest")
    if manifest.get("blocks") != [name for _, _, name in expected_blocks()]:
        raise ValueError("manifest block order changed")
    records = [
        json.loads(line)
        for line in (directory / "run-order.jsonl").read_text().splitlines()
    ]
    check_order(records)
    if json.loads((directory / "campaign-summary.json").read_text()) != {
        "status": "VALID_BASELINE_NOISE_PILOT",
        "valid_blocks": 10,
        "planned_blocks": 10,
    }:
        raise ValueError("producer campaign summary differs from ten valid blocks")
    result_blocks = []
    by_size: dict[int, dict[str, list[dict[str, Any]]]] = {32: {}, 256: {}}
    for record, (size, _, name) in zip(records, expected_blocks()):
        block_dir = directory / name
        attempt = json.loads((block_dir / "attempt.json").read_text())
        if (
            record.get("status") != "VALID_BASELINE_S03_PILOT_BLOCK"
            or attempt.get("status") != record["status"]
        ):
            raise ValueError(f"{name}: invalid or mismatched block status")
        if record.get("invalidators") != [] or attempt.get("invalidators") != []:
            raise ValueError(f"{name}: invalidators present")
        protocol = json.loads((block_dir / "pilot-protocol.json").read_text())
        if protocol.get("size") != size or protocol.get("repetitions_per_case") != 100:
            raise ValueError(f"{name}: block protocol changed")
        if attempt.get("protocol_sha256") != sample_audit.digest(
            block_dir / "pilot-protocol.json"
        ):
            raise ValueError(f"{name}: block protocol hash changed")
        recomputed = sample_audit.audit(
            block_dir / "workload",
            size=size,
            source_sha=SOURCE,
            wheel_sha256=WHEEL_SHA256,
        )
        retained = json.loads((block_dir / "independent-audit.json").read_text())
        if retained != recomputed or attempt.get(
            "summary_sha256"
        ) != sample_audit.digest(block_dir / "independent-audit.json"):
            raise ValueError(f"{name}: independent audit claim changed")
        if any(cell["count"] != 100 for cell in recomputed["cases"].values()):
            raise ValueError(f"{name}: fewer than 100 semantic observations")
        resources = _resources(block_dir)
        warnings = _environment(block_dir)
        if warnings != attempt.get("warnings"):
            raise ValueError(f"{name}: warning claim differs from environment")
        for cell, values in recomputed["cases"].items():
            by_size[size].setdefault(cell, []).append(values)
        result_blocks.append(
            {
                "name": name,
                "raw_sha256": recomputed["raw_sha256"],
                "database_sha256": recomputed["database_sha256"],
                "resources": resources,
                "warnings": warnings,
            }
        )
    noise = {}
    for size, cells in by_size.items():
        noise[str(size)] = {}
        for name, values in cells.items():
            if len(values) != 5:
                raise ValueError(f"{size}/{name}: fewer than five blocks")
            p50 = [value["p50_ns"] for value in values]
            p95 = [value["p95_ns"] for value in values]
            noise[str(size)][name] = {
                "block_p50_ns": p50,
                "block_p95_ns": p95,
                "p50_span_pct_of_median": (max(p50) - min(p50))
                / statistics.median(p50)
                * 100,
                "p95_span_pct_of_median": (max(p95) - min(p95))
                / statistics.median(p95)
                * 100,
            }
    return {
        "status": "AUDITED_S03_BASELINE_NOISE_PILOT_NOT_PAIRED_COMPARISON",
        "source_sha": SOURCE,
        "manifest_sha256": sample_audit.digest(directory / "pilot-manifest.json"),
        "run_order_sha256": sample_audit.digest(directory / "run-order.jsonl"),
        "blocks": result_blocks,
        "noise": noise,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(audit(args.input), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
