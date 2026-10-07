#!/usr/bin/env python3
"""Independently recompute installed-Python S02-L lifecycle blocks."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
import zipfile


ROOT = Path(__file__).resolve().parents[1]
BLOCK_RUNNER = ROOT / "scripts/slice135_python_close_block.py"
RUNNER = ROOT / "scripts/slice135_python_close_memory.py"
PILOT_HELPER = ROOT / "scripts/slice135_pilot.py"
STABLE = ("host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler")
MEMORY = ("rss_kib", "pss_kib", "private_kib")
RESOURCE = {
    "user_s": "user_cpu_s",
    "system_s": "system_cpu_s",
    "peak_rss_kib": "peak_rss_kib",
    "fs_inputs": "fs_inputs",
    "fs_outputs": "fs_outputs",
    "major_faults": "major_faults",
    "swap_events": "swap_events",
}


def sha(path: Path) -> str:
    """Hash exact file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nearest_rank(values: list[int], quantile: float) -> int:
    """Recompute the untrimmed nearest-rank percentile."""
    if not values or not 0 < quantile <= 1:
        raise ValueError("nearest-rank input invalid")
    return sorted(values)[math.ceil(quantile * len(values)) - 1]


def pair_delta(baseline: int, candidate: int) -> float:
    """Return a signed percent change from the baseline."""
    if baseline <= 0 or candidate <= 0:
        raise ValueError("paired sample invalid")
    return 100 * (candidate - baseline) / baseline


def check_sample(raw: dict, manifest: dict) -> tuple[int, int]:
    """Verify the measured lifecycle and its artifact without producer code."""
    if (
        raw.get("schema_version") != 1
        or raw.get("source_sha") != manifest["source_sha"]
        or raw.get("runner_sha256") != manifest["runner_sha256"]
    ):
        raise ValueError("source or runner identity changed")
    artifact = raw.get("artifact")
    if not isinstance(artifact, dict) or any(
        artifact.get(key) != manifest[key] for key in ("wheel_sha256", "native_sha256")
    ):
        raise ValueError("wheel or native identity changed")
    if raw.get("host", {}).get("embed_device") != manifest["embed_device"]:
        raise ValueError("embed device changed")
    embedder = raw.get("embedder", {})
    if (
        embedder.get("name") != manifest["embedder_name"]
        or embedder.get("dimension") != 384
    ):
        raise ValueError("default embedder changed")
    if (
        raw.get("semantic_ok") is not True
        or raw.get("reopen_ok") is not True
        or raw.get("vector_dimension") != 384
        or raw.get("vector_nonzero") is not True
    ):
        raise ValueError("lifecycle or vector assertion failed")
    for field in ("open_embed_ns", "close_ns"):
        if type(raw.get(field)) is not int or raw[field] <= 0:
            raise ValueError(f"{field} invalid")
    for state in ("before", "opened", "closed", "closed_idle", "dropped"):
        values = raw.get(state)
        if not isinstance(values, dict) or set(values) != set(MEMORY):
            raise ValueError(f"{state} memory missing")
        if any(type(values[key]) is not int or values[key] <= 0 for key in MEMORY):
            raise ValueError(f"{state} memory invalid")
        if (
            values["pss_kib"] > values["rss_kib"]
            or values["private_kib"] > values["rss_kib"]
        ):
            raise ValueError(f"{state} memory impossible")
    return raw["close_ns"], raw["opened"]["pss_kib"] - raw["closed_idle"]["pss_kib"]


def check_resource(path: Path, recorded: dict) -> None:
    """Reparse GNU Time output and compare every recorded counter."""
    fields = dict(
        line.split("=", 1) for line in path.read_text().splitlines() if "=" in line
    )
    if recorded.get("method") != "gnu-time" or recorded.get("unsupported") != []:
        raise ValueError("child resource report incomplete")
    for field, key in RESOURCE.items():
        value = (
            float(fields[field])
            if field in ("user_s", "system_s")
            else int(fields[field])
        )
        if recorded.get(key) != value:
            raise ValueError(f"child {key} differs from GNU Time")


def check_environment(environment: dict, resources: list[dict]) -> list[str]:
    """Recompute host invalidators and paging warnings."""
    start, end = environment["start"], environment["end"]
    if any(not start.get(key) or start[key] != end.get(key) for key in STABLE):
        raise ValueError("host identity drift")
    if (
        start["governor"] == "unavailable"
        or start["competing_jobs"]
        or end["competing_jobs"]
        or min(start["disk_free_bytes"], end["disk_free_bytes"]) < 1_073_741_824
    ):
        raise ValueError("host invalid at lifecycle block")
    if end["swap_pages"] < start["swap_pages"] or any(
        resource["swap_events"] for resource in resources
    ):
        raise ValueError("child swap or host counter reversal")
    if environment["invalidators"]:
        raise ValueError("environment invalidator retained")
    warnings = []
    delta = end["swap_pages"] - start["swap_pages"]
    if delta:
        warnings.append(f"host swap drift: {delta} pages; child swap events: 0")
    for resource in resources:
        if resource["major_faults"]:
            warnings.append(f"child major faults: {resource['major_faults']}")
    if sorted(set(warnings)) != sorted(environment["warnings"]):
        raise ValueError("environment warnings differ from counters")
    return sorted(set(warnings))


def audit_block(directory: Path, wheel: Path) -> dict:
    """Verify one block's original files and recompute its summary."""
    manifest_path = directory / "manifest.json"
    environment_path = directory / "environment.json"
    attempts_path = directory / "attempts.json"
    summary_path = directory / "summary.json"
    manifest = json.loads(manifest_path.read_text())
    environment = json.loads(environment_path.read_text())
    attempts = json.loads(attempts_path.read_text())
    summary = json.loads(summary_path.read_text())
    if (
        manifest.get("schema_version") != 1
        or manifest.get("role") not in ("baseline", "candidate")
        or manifest.get("warmups") != 1
        or manifest.get("samples", 0) < 3
        or manifest.get("embed_device") != "cpu"
        or manifest.get("embedder_name") != "fathomdb-bge-small-en-v1.5"
    ):
        raise ValueError("manifest declaration invalid")
    if (
        sha(wheel) != manifest["wheel_sha256"]
        or sha(directory / "bundle" / RUNNER.name) != manifest["runner_sha256"]
        or sha(RUNNER) != manifest["runner_sha256"]
        or sha(BLOCK_RUNNER) != manifest["block_runner_sha256"]
        or sha(PILOT_HELPER) != manifest["pilot_helper_sha256"]
    ):
        raise ValueError("wheel or runner byte identity changed")
    with zipfile.ZipFile(wheel) as archive:
        native_sha256 = hashlib.sha256(
            archive.read("fathomdb/_fathomdb.abi3.so")
        ).hexdigest()
    if native_sha256 != manifest["native_sha256"]:
        raise ValueError("native wheel member changed")
    labels = ["warmup", *[f"sample-{i:03d}" for i in range(1, manifest["samples"] + 1)]]
    if [attempt.get("label") for attempt in attempts] != labels:
        raise ValueError("attempt count, order or labels changed")
    close_samples = []
    release_samples = []
    resources = []
    for attempt in attempts:
        label = attempt["label"]
        raw_path = directory / f"{label}.json"
        resource_path = directory / f"{label}.resource.txt"
        if attempt["raw_sha256"] != sha(raw_path):
            raise ValueError(f"{label} raw hash changed")
        raw = json.loads(raw_path.read_text())
        close_ns, release_kib = check_sample(raw, manifest)
        check_resource(resource_path, attempt["resource"])
        if (
            attempt["valid"] is not True
            or attempt["invalidators"] != []
            or attempt["close_ns"] != close_ns
            or attempt["pss_release_kib"] != release_kib
        ):
            raise ValueError(f"{label} validity or summary changed")
        resources.append(attempt["resource"])
        if label != "warmup":
            close_samples.append(close_ns)
            release_samples.append(release_kib)
    warnings = check_environment(environment, resources)
    expected = {
        "schema_version": 1,
        "status": "VALID_S02_L_BLOCK",
        "role": manifest["role"],
        "invalidators": [],
        "warnings": warnings,
        "manifest_sha256": sha(manifest_path),
        "environment_sha256": sha(environment_path),
        "attempts_sha256": sha(attempts_path),
        "valid_samples": manifest["samples"],
        "p50_close_ns": nearest_rank(close_samples, 0.5),
        "p95_close_ns": nearest_rank(close_samples, 0.95),
        "p50_pss_release_kib": nearest_rank(release_samples, 0.5),
        "min_pss_release_kib": min(release_samples),
        "max_pss_release_kib": max(release_samples),
    }
    if summary != expected:
        raise ValueError("producer summary differs from recomputation")
    return {
        "block": directory.name,
        "source_sha": manifest["source_sha"],
        "wheel_sha256": manifest["wheel_sha256"],
        "runner_sha256": manifest["runner_sha256"],
        "role": manifest["role"],
        "samples": len(close_samples),
        "close_p50_ns": expected["p50_close_ns"],
        "close_p95_ns": expected["p95_close_ns"],
        "close_min_ns": min(close_samples),
        "close_max_ns": max(close_samples),
        "pss_release_p50_kib": expected["p50_pss_release_kib"],
        "warnings": warnings,
        "start_utc": environment["start"]["time_utc"],
        "end_utc": environment["end"]["time_utc"],
        "raw_close_samples_ns": close_samples,
        "raw_pss_release_samples_kib": release_samples,
    }


def audit_pilot(directories: list[Path], wheel: Path) -> dict:
    """Verify five baseline blocks and report noise without selecting outcomes."""
    blocks = [audit_block(path, wheel) for path in directories]
    if len(blocks) < 5 or any(block["role"] != "baseline" for block in blocks):
        raise ValueError("five baseline-only blocks required")
    if any(block["samples"] < 20 for block in blocks):
        raise ValueError("baseline pilot has fewer than 100 valid cycles")
    for key in ("source_sha", "wheel_sha256", "runner_sha256"):
        if len({block[key] for block in blocks}) != 1:
            raise ValueError(f"baseline pilot {key} drift")
    starts = [block["start_utc"] for block in blocks]
    if starts != sorted(starts):
        raise ValueError("block order differs from start time")
    p50s = [block["close_p50_ns"] for block in blocks]
    p95s = [block["close_p95_ns"] for block in blocks]
    return {
        "schema_version": 1,
        "status": "VALID_BASELINE_S02_L_NOISE_PILOT",
        "source_sha": blocks[0]["source_sha"],
        "wheel_sha256": blocks[0]["wheel_sha256"],
        "runner_sha256": blocks[0]["runner_sha256"],
        "block_count": len(blocks),
        "valid_cycles": sum(block["samples"] for block in blocks),
        "block_p50_spread_ns": max(p50s) - min(p50s),
        "block_p95_spread_ns": max(p95s) - min(p95s),
        "first_last_p50_delta_percent": pair_delta(p50s[0], p50s[-1]),
        "block_p50_median_ns": statistics.median_low(p50s),
        "blocks": blocks,
    }


def main() -> None:
    """Write an independent, non-overwriting baseline pilot audit."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("blocks", type=Path, nargs="+")
    args = parser.parse_args()
    if args.output.exists():
        parser.error("refusing to overwrite audit")
    result = audit_pilot(args.blocks, args.wheel)
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
