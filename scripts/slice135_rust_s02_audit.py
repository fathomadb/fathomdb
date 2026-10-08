#!/usr/bin/env python3
"""Independently audit retained real-database Rust SDK S02 timing blocks."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import sqlite3
import subprocess


SOURCE_SHA = "3f29d649d0213e595c0dab251a449d92fd625792"
RUST_TREE = "c8eaaa19a0e0e876326b778733ace354cc7e21e8"
STAGES = frozenset({
    "open", "write", "configure_projection", "drain", "text",
    "vector_lexical_control", "vector", "hybrid", "evidence_search",
    "evidence_resolve", "graph_expand", "graph_target_resolve",
    "graph_edge_resolve", "graph_neighbors", "erase", "erase_again",
    "close", "reopen", "reopened_close",
})
EXPECTED_OBSERVED = {
    "corpus_nodes": 32,
    "erased_graph_nodes": 3,
    "erased_graph_edges": 1,
    "reopened_graph_absent": True,
    "reopened_anchor_retained": True,
    "evidence_resolved": True,
    "projection_ready": True,
}
EXPECTED_SQLITE = {
    "integrity_check": "ok", "corpus_nodes": 32,
    "graph_nodes": 0, "graph_edges": 0,
}
RESOURCE_FIELDS = {
    "user_s": "user_cpu_s", "system_s": "system_cpu_s",
    "peak_rss_kib": "peak_rss_kib", "fs_inputs": "fs_inputs",
    "fs_outputs": "fs_outputs", "major_faults": "major_faults",
    "swap_events": "swap_events",
}


def sha(path: Path) -> str:
    """Hash retained bytes without consulting producer summaries."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_json(path: Path) -> dict | list:
    """Read one raw evidence file."""
    return json.loads(path.read_text())


def validate_raw(raw: dict) -> int:
    """Validate one consumer's product timer and semantic assertions."""
    if not isinstance(raw, dict) or raw.get("schema_version") != 1:
        raise ValueError("Rust S02 receipt schema")
    if raw.get("source_sha") != SOURCE_SHA or raw.get("status") != "TIMED_RUST_SDK_S02_FUNCTIONAL_OK":
        raise ValueError("Rust S02 source/status")
    stages = raw.get("stage_ns")
    if not isinstance(stages, dict) or set(stages) != STAGES:
        raise ValueError("Rust S02 stage set")
    if any(type(value) is not int or value <= 0 for value in stages.values()):
        raise ValueError("Rust S02 stage duration")
    whole = raw.get("whole_product_ns")
    if type(whole) is not int or whole < sum(stages.values()):
        raise ValueError("Rust S02 whole product duration")
    observed = raw.get("observed")
    if not isinstance(observed, dict):
        raise ValueError("Rust S02 observed state")
    for key, expected in EXPECTED_OBSERVED.items():
        if type(observed.get(key)) is not type(expected) or observed[key] != expected:
            raise ValueError(f"Rust S02 {key} differs from expected state")
    if set(observed) != set(EXPECTED_OBSERVED):
        raise ValueError("Rust S02 observed state keys")
    return whole


def sqlite_state(path: Path) -> dict:
    """Inspect the retained database using a fresh independent connection."""
    with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
        return {
            "integrity_check": connection.execute("PRAGMA integrity_check").fetchone()[0],
            "corpus_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-rust-corpus'"
            ).fetchone()[0],
            "graph_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-rust-graph'"
            ).fetchone()[0],
            "graph_edges": connection.execute(
                "SELECT COUNT(*) FROM canonical_edges WHERE source_id='slice135-rust-graph'"
            ).fetchone()[0],
        }


def resource_report(path: Path) -> dict:
    """Parse GNU Time text without using the timing producer's parser."""
    pairs = dict(line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)
    if set(pairs) != set(RESOURCE_FIELDS):
        raise ValueError("Rust S02 resource report fields")
    return {
        recorded: float(pairs[key]) if key in {"user_s", "system_s"} else int(pairs[key])
        for key, recorded in RESOURCE_FIELDS.items()
    }


def nearest_rank(values: list[int], fraction: float) -> int:
    """Recompute nearest-rank percentiles from retained raw attempts."""
    return sorted(values)[math.ceil(fraction * len(values)) - 1]


def audit_block(block: Path, checkout: Path, crate: Path, runner: Path, binary: Path) -> dict:
    """Reject any raw/state/resource/identity disagreement in a timing block."""
    summary = read_json(block / "summary.json")
    attempts = read_json(block / "attempts.json")
    environment = read_json(block / "environment.json")
    identity = summary["identity"]
    if subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=checkout, check=True,
        capture_output=True, text=True,
    ).stdout.strip() != SOURCE_SHA:
        raise ValueError("Rust S02 checkout source SHA")
    if subprocess.run(
        ["git", "rev-parse", "HEAD:src/rust/crates"], cwd=checkout,
        check=True, capture_output=True, text=True,
    ).stdout.strip() != RUST_TREE:
        raise ValueError("Rust S02 checkout crate tree")
    if subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=all"], cwd=checkout,
        check=True, capture_output=True, text=True,
    ).stdout.strip():
        raise ValueError("Rust S02 checkout has uncommitted files")
    if str(checkout / "src/rust/crates/fathomdb-sdk") not in (crate / "Cargo.toml").read_text():
        raise ValueError("Rust S02 manifest does not select the measured SDK")
    expected_identity = {
        "product_source_sha": SOURCE_SHA,
        "rust_crates_tree": RUST_TREE,
        "runner_sha256": sha(runner),
        "manifest_sha256": sha(crate / "Cargo.toml"),
        "external_lock_sha256": sha(crate / "Cargo.lock"),
        "binary_sha256": sha(binary),
    }
    if identity != expected_identity or sha(crate / "src/main.rs") != sha(runner):
        raise ValueError("Rust S02 identity or runner bytes")
    if summary["attempts_sha256"] != sha(block / "attempts.json") or summary[
        "environment_sha256"
    ] != sha(block / "environment.json"):
        raise ValueError("Rust S02 raw file hash")
    if not isinstance(attempts, list) or len(attempts) != summary["samples"] + 1:
        raise ValueError("Rust S02 sample count")
    if summary["warmups"] != 1 or summary["samples"] < 3:
        raise ValueError("Rust S02 warmup or sample minimum")
    if environment["invalidators"] or summary["invalidators"]:
        raise ValueError("Rust S02 environment invalidator")
    start, end = environment["start"], environment["end"]
    for name in ("host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler"):
        if not start.get(name) or start[name] != end.get(name):
            raise ValueError(f"Rust S02 environment {name}")
    if start["competing_jobs"] or end["competing_jobs"]:
        raise ValueError("Rust S02 competing jobs")
    if min(start["disk_free_bytes"], end["disk_free_bytes"]) < 1_073_741_824:
        raise ValueError("Rust S02 disk space")
    warnings = []
    drift = end["swap_pages"] - start["swap_pages"]
    if drift:
        warnings.append(f"host swap drift: {drift} pages; child swap events: 0")
    if summary["warnings"] != warnings or environment["warnings"] != warnings:
        raise ValueError("Rust S02 environment warning")
    values: list[int] = []
    max_rss = 0
    for number, attempt in enumerate(attempts):
        label = "warmup" if number == 0 else f"sample-{number:03d}"
        if attempt["label"] != label or attempt["exit_code"] != 0 or attempt["valid"] is not True:
            raise ValueError("Rust S02 attempt order or validity")
        stdout = block / f"{label}.stdout"
        stderr = block / f"{label}.stderr"
        database = block / f"{label}.sqlite"
        if attempt["stdout_sha256"] != sha(stdout) or attempt["stderr_sha256"] != sha(stderr):
            raise ValueError("Rust S02 attempt stdout/stderr hash")
        command = read_json(block / f"{label}.command.json")
        if command[-2:] != [str(binary), str(database)] or "-f" not in command:
            raise ValueError("Rust S02 attempt command")
        lines = stdout.read_text().splitlines()
        if len(lines) != 1 or not lines[0].startswith("SLICE135_RUST_S02 "):
            raise ValueError("Rust S02 receipt line")
        raw = json.loads(lines[0].split(" ", 1)[1])
        duration = validate_raw(raw)
        if duration != attempt["whole_product_ns"]:
            raise ValueError("Rust S02 recorded whole duration")
        resource = resource_report(block / f"{label}.resource.txt")
        if any(attempt["resource"].get(key) != value for key, value in resource.items()):
            raise ValueError("Rust S02 recorded resource report")
        if attempt["resource"].get("scope") != "measured-workload-child-process" or attempt[
            "resource"
        ].get("method") != "gnu-time" or attempt["resource"].get("unsupported") != []:
            raise ValueError("Rust S02 resource metadata")
        if resource["swap_events"]:
            raise ValueError("Rust S02 child swap events")
        state = sqlite_state(database)
        if state != EXPECTED_SQLITE or attempt["observed_state"] != state:
            raise ValueError("Rust S02 reopened SQLite state")
        if attempt["database_sha256"] != sha(database):
            raise ValueError("Rust S02 database hash")
        max_rss = max(max_rss, resource["peak_rss_kib"])
        if number:
            values.append(duration)
    expected_statistics = {
        "valid_samples": len(values),
        "p50_ns": nearest_rank(values, 0.5),
        "p95_ns": nearest_rank(values, 0.95) if len(values) >= 20 else None,
        "min_ns": min(values), "max_ns": max(values),
    }
    if any(summary[key] != value for key, value in expected_statistics.items()):
        raise ValueError("Rust S02 statistics")
    if summary["status"] != "VALID_RUST_S02_TIMING_BLOCK":
        raise ValueError("Rust S02 block status")
    return {
        "status": "ACCEPTED_RUST_S02_TIMING_BLOCK",
        "block": str(block),
        "source_sha": SOURCE_SHA,
        "samples": len(values),
        "p50_ns": expected_statistics["p50_ns"],
        "p95_ns": expected_statistics["p95_ns"],
        "peak_rss_kib": max_rss,
        "host_swap_drift_pages": drift,
        "summary_sha256": sha(block / "summary.json"),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--block", type=Path, required=True)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--crate", type=Path, required=True)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit_block(
        args.block.resolve(), args.checkout.resolve(), args.crate.resolve(),
        args.runner.resolve(), args.binary.resolve(),
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"])


if __name__ == "__main__":
    main()
