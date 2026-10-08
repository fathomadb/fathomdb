#!/usr/bin/env python3
"""Capture one real-database candidate-only Rust SDK S02 timing block."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import sqlite3
import subprocess
import sys

import slice135_pilot as pilot


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "scripts/slice135_rust_s02_timing.rs"
SOURCE_SHA = "3f29d649d0213e595c0dab251a449d92fd625792"
RUST_TREE = "c8eaaa19a0e0e876326b778733ace354cc7e21e8"
MIN_DISK_FREE_BYTES = 1_073_741_824
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


def sha(path: Path) -> str:
    """Hash exact file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(checkout: Path, *arguments: str) -> str:
    """Read one Git identity from the clean measured source checkout."""
    return subprocess.run(
        ["git", *arguments], cwd=checkout, capture_output=True, text=True,
        check=True,
    ).stdout.strip()


def verify_source(checkout: Path, crate: Path, binary: Path) -> dict:
    """Bind the external consumer, binary and product source before timing."""
    if git(checkout, "rev-parse", "HEAD") != SOURCE_SHA or git(
        checkout, "status", "--porcelain", "--untracked-files=all"
    ):
        raise ValueError("Rust S02 measured source checkout differs from clean freeze")
    if git(checkout, "rev-parse", "HEAD:src/rust/crates") != RUST_TREE:
        raise ValueError("Rust S02 product tree differs from freeze")
    if sha(crate / "src/main.rs") != sha(SOURCE):
        raise ValueError("Rust S02 external consumer differs from runner source")
    manifest = (crate / "Cargo.toml").read_text()
    if str(checkout / "src/rust/crates/fathomdb-sdk") not in manifest:
        raise ValueError("Rust S02 manifest does not resolve exact candidate SDK")
    if not binary.is_file():
        raise ValueError("Rust S02 executable absent")
    return {
        "product_source_sha": SOURCE_SHA,
        "rust_crates_tree": RUST_TREE,
        "runner_sha256": sha(SOURCE),
        "manifest_sha256": sha(crate / "Cargo.toml"),
        "external_lock_sha256": sha(crate / "Cargo.lock"),
        "binary_sha256": sha(binary),
    }


def parse_receipt(stdout: str) -> dict:
    """Reject wrong product timing, missing stages or false functional state."""
    lines = stdout.splitlines()
    if len(lines) != 1 or not lines[0].startswith("SLICE135_RUST_S02 "):
        raise ValueError("Rust S02 structured receipt missing")
    raw = json.loads(lines[0].split(" ", 1)[1])
    if (
        not isinstance(raw, dict)
        or raw.get("schema_version") != 1
        or raw.get("status") != "TIMED_RUST_SDK_S02_FUNCTIONAL_OK"
        or raw.get("source_sha") != SOURCE_SHA
    ):
        raise ValueError("Rust S02 source or receipt schema changed")
    stages = raw.get("stage_ns")
    if (
        not isinstance(stages, dict)
        or set(stages) != STAGES
        or any(type(value) is not int or value <= 0 for value in stages.values())
    ):
        raise ValueError("Rust S02 stage durations invalid")
    whole = raw.get("whole_product_ns")
    if type(whole) is not int or whole < sum(stages.values()):
        raise ValueError("Rust S02 whole product duration invalid")
    if raw.get("observed") != EXPECTED_OBSERVED:
        raise ValueError("Rust S02 observed state differs from contract")
    return raw


def reopened_state(database: Path) -> dict:
    """Read persisted state through an independent SQLite connection."""
    with sqlite3.connect(database) as connection:
        result = {
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
    if result != {
        "integrity_check": "ok", "corpus_nodes": 32,
        "graph_nodes": 0, "graph_edges": 0,
    }:
        raise ValueError("Rust S02 reopened database state differs from contract")
    return result


def nearest_rank(values: list[int], quantile: float) -> int:
    """Return the predeclared nearest-rank sample percentile."""
    return sorted(values)[math.ceil(len(values) * quantile) - 1]


def run_block(
    *, checkout: Path, crate: Path, binary: Path, samples: int, output: Path,
) -> dict:
    """Retain every fresh-process sequence, resource report and state oracle."""
    if samples < 3:
        raise ValueError("Rust S02 block needs at least three measured sequences")
    identity = verify_source(checkout, crate, binary)
    output.mkdir(parents=True, exist_ok=False)
    start = pilot.inventory(output)
    attempts = []
    environment = {**os.environ, "FATHOMDB_EMBED_DEVICE": "cpu", "HF_HUB_OFFLINE": "1"}
    for number in range(samples + 1):
        label = "warmup" if number == 0 else f"sample-{number:03d}"
        database = output / f"{label}.sqlite"
        resource_path = output / f"{label}.resource.txt"
        command = [
            str(pilot.GNU_TIME), "-o", str(resource_path), "-f", pilot.RESOURCE_FORMAT,
            str(binary), str(database),
        ]
        try:
            child = subprocess.run(
                command, capture_output=True, text=True, env=environment,
                check=False, timeout=120,
            )
            stdout, stderr, exit_code = child.stdout, child.stderr, child.returncode
        except subprocess.TimeoutExpired as error:
            stdout = (error.stdout or b"").decode(errors="replace")
            stderr = (error.stderr or b"").decode(errors="replace")
            exit_code = 124
        (output / f"{label}.stdout").write_text(stdout)
        (output / f"{label}.stderr").write_text(stderr)
        (output / f"{label}.command.json").write_text(json.dumps(command) + "\n")
        resource = pilot.read_resource_report(resource_path)
        attempt = {
            "label": label, "exit_code": exit_code,
            "stdout_sha256": sha(output / f"{label}.stdout"),
            "stderr_sha256": sha(output / f"{label}.stderr"),
            "resource": resource,
        }
        try:
            if exit_code:
                raise ValueError(f"Rust S02 process exited {exit_code}")
            raw = parse_receipt(stdout)
            state = reopened_state(database)
            if not pilot.complete_child_resources(resource) or resource["swap_events"]:
                raise ValueError("Rust S02 child resource or swap invalid")
            attempt.update({
                "valid": True, "whole_product_ns": raw["whole_product_ns"],
                "observed_state": state,
                "database_sha256": sha(database),
            })
        except (ValueError, KeyError, sqlite3.Error, json.JSONDecodeError) as error:
            attempt.update({"valid": False, "reason": str(error)})
        attempts.append(attempt)
        if not attempt["valid"]:
            break
    end = pilot.inventory(output)
    resources = [item["resource"] for item in attempts]
    invalidators = []
    for resource in resources:
        invalidators.extend(pilot.environment_invalidators(
            start, end, MIN_DISK_FREE_BYTES, resource,
        ))
    invalidators = sorted(set(invalidators))
    warnings = sorted(set(
        warning
        for resource in resources
        for warning in pilot.environment_warnings(start, end, resource)
    ))
    environment_receipt = {
        "start": start, "end": end,
        "invalidators": invalidators, "warnings": warnings,
    }
    (output / "environment.json").write_text(json.dumps(environment_receipt, indent=2) + "\n")
    (output / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
    values = [item["whole_product_ns"] for item in attempts[1:] if item["valid"]]
    valid = len(attempts) == samples + 1 and all(item["valid"] for item in attempts)
    summary = {
        "status": "VALID_RUST_S02_TIMING_BLOCK" if valid and not invalidators else "INVALID_RUST_S02_TIMING_BLOCK",
        "identity": identity,
        "warmups": 1, "samples": samples, "valid_samples": len(values),
        "invalidators": invalidators, "warnings": warnings,
        "attempts_sha256": sha(output / "attempts.json"),
        "environment_sha256": sha(output / "environment.json"),
        "p50_ns": nearest_rank(values, 0.5) if values else None,
        "p95_ns": nearest_rank(values, 0.95) if len(values) >= 20 else None,
        "min_ns": min(values) if values else None,
        "max_ns": max(values) if values else None,
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", required=True, type=Path)
    parser.add_argument("--crate", required=True, type=Path)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--samples", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    summary = run_block(
        checkout=args.checkout.resolve(), crate=args.crate.resolve(),
        binary=args.binary.resolve(), samples=args.samples,
        output=args.output.absolute(),
    )
    print(summary["status"], flush=True)
    if summary["status"] != "VALID_RUST_S02_TIMING_BLOCK":
        sys.exit(1)


if __name__ == "__main__":
    main()
