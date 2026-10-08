#!/usr/bin/env python3
"""Run the frozen E01–E12 engine pairs on the integrated 0.8.27 candidate."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
DEFAULT_FREEZE = PLAN / "e12-integrated-comparison-protocol.json"
EXPECTED = PLAN / "e12-expected-checks.json"
MODEL = (
    Path.home()
    / ".cache/huggingface/hub/models--BAAI--bge-small-en-v1.5"
    / "snapshots/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"
)
SHA40 = re.compile(r"[0-9a-f]{40}")
CELLS = {
    "query": ["text", "vector_stage", "hybrid", "graph_expand", "graph_evidence"],
    "lifecycle": [
        "open_fresh", "open_populated", "close", "canonical_write", "projection",
        "model_cpu", "erasure",
    ],
}


def candidate_sha(frozen: dict) -> str:
    """Require a concrete candidate identity supplied by the frozen protocol."""
    value = frozen.get("candidate_product_snapshot", {}).get("source_sha_before_protocol_commit")
    if not isinstance(value, str) or not SHA40.fullmatch(value) or int(value, 16) == 0:
        raise ValueError("candidate source identity is invalid")
    return value


def planned_blocks(frozen: dict) -> list[dict]:
    """Expand the exact frozen cell, sample and alternation schedule."""
    if frozen.get("status") != "FROZEN_E01_E12_PAIRED_SUBSET":
        raise ValueError("E01–E12 subset is not frozen")
    candidate_sha(frozen)
    workload = frozen["workload"]
    for group, cells in CELLS.items():
        if workload.get(f"{group}_cells") != cells:
            raise ValueError(f"{group} cells differ from freeze")
    if workload.get("query_samples_per_block") != 1000 or workload.get("lifecycle_samples_per_block") != 100:
        raise ValueError("E01–E12 samples differ from freeze")
    if workload.get("pair_count") != 5 or len(workload.get("pair_order", [])) != 5:
        raise ValueError("E01–E12 pair count differs from freeze")
    blocks = []
    for group, cells in CELLS.items():
        samples = workload[f"{group}_samples_per_block"]
        for pair, order in enumerate(workload["pair_order"], start=1):
            if len(order) != 2 or sorted(order) != ["baseline", "candidate"]:
                raise ValueError(f"E01–E12 pair {pair} lacks both versions")
            for position, version in enumerate(order, start=1):
                blocks.append({
                    "group": group,
                    "pair": pair,
                    "position": position,
                    "version": version,
                    "name": f"{group}-pair-{pair}-{position}-{version}",
                    "samples": samples,
                    "cells": cells,
                })
    return blocks


def sha(path: Path) -> str:
    """Hash one frozen or executable source file."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(checkout: Path, object_name: str) -> str:
    """Resolve one source object from a dedicated clean checkout."""
    return subprocess.check_output(["git", "rev-parse", object_name], cwd=checkout, text=True).strip()


def check_inputs(frozen: dict, baseline: Path, candidate: Path) -> None:
    """Reject a wrong source, changed workload or insufficient disk before timing."""
    if git(baseline, "HEAD") != frozen["baseline"]["source_sha"]:
        raise ValueError("baseline checkout differs from freeze")
    if git(candidate, "HEAD") != candidate_sha(frozen):
        raise ValueError("candidate checkout differs from freeze")
    for name, expected in (
        ("HEAD:src/rust/crates", frozen["candidate_product_snapshot"]["rust_crates_git_tree"]),
        ("HEAD:Cargo.lock", frozen["candidate_product_snapshot"]["cargo_lock_git_blob"]),
    ):
        if git(candidate, name) != expected:
            raise ValueError(f"candidate {name} differs from freeze")
    for key, path in (
        ("adapter_sha256", ROOT / "scripts/slice135_e12_adapter.py"),
        ("workload_sha256", ROOT / "scripts/slice135_e12_workload.rs"),
        ("expected_checks_sha256", EXPECTED),
    ):
        if sha(path) != frozen["runner_sources"][key]:
            raise ValueError(f"{key} changed")
    if shutil.disk_usage("/tmp").free < frozen["validity"]["minimum_disk_free_bytes"]:
        raise ValueError("insufficient disk for E01–E12 campaign")


def run(command: list[str], log_base: Path) -> None:
    """Retain the full output of one producer or independent audit command."""
    started = time.monotonic()
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False, timeout=3600)
    log_base.with_suffix(".stdout.log").write_text(result.stdout)
    log_base.with_suffix(".stderr.log").write_text(result.stderr)
    print(json.dumps({
        "command": command,
        "exit": result.returncode,
        "elapsed_s": round(time.monotonic() - started, 3),
        "stdout_tail": result.stdout.strip()[-500:],
        "stderr_tail": result.stderr.strip()[-500:],
    }), flush=True)
    if result.returncode:
        raise RuntimeError(f"{log_base.name} exited {result.returncode}")


def main() -> None:
    """Run every frozen pair once and preserve invalid attempts without replacement."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--baseline-checkout", required=True, type=Path)
    parser.add_argument("--candidate-checkout", required=True, type=Path)
    parser.add_argument("--baseline-binary", required=True, type=Path)
    parser.add_argument("--candidate-binary", required=True, type=Path)
    parser.add_argument("--freeze", type=Path, default=DEFAULT_FREEZE)
    args = parser.parse_args()
    output = args.output.resolve()
    frozen_bytes = args.freeze.read_bytes()
    frozen = json.loads(frozen_bytes)
    blocks = planned_blocks(frozen)
    check_inputs(frozen, args.baseline_checkout, args.candidate_checkout)
    output.mkdir(parents=True, exist_ok=False)
    manifest = {
        "freeze_sha256": hashlib.sha256(frozen_bytes).hexdigest(),
        "baseline_source_sha": git(args.baseline_checkout, "HEAD"),
        "candidate_source_sha": git(args.candidate_checkout, "HEAD"),
        "minimum_idle_seconds": frozen["workload"]["minimum_idle_seconds_between_blocks"],
        "campaign_runner_sha256": sha(Path(__file__)),
        "blocks": [],
    }
    schedule = output / "schedule.json"
    schedule.write_text(json.dumps(manifest, indent=2) + "\n")
    checkouts = {"baseline": args.baseline_checkout, "candidate": args.candidate_checkout}
    binaries = {"baseline": args.baseline_binary, "candidate": args.candidate_binary}
    time.sleep(manifest["minimum_idle_seconds"])
    for index, block in enumerate(blocks):
        name, version = block["name"], block["version"]
        entry = {key: block[key] for key in ("group", "pair", "position", "version", "name")}
        entry.update({"start_unix": time.time(), "status": "started"})
        manifest["blocks"].append(entry)
        schedule.write_text(json.dumps(manifest, indent=2) + "\n")
        try:
            run(
                [sys.executable, "scripts/slice135_e12_adapter.py", "--checkout", str(checkouts[version]),
                 "--output", str(output / name), "--binary", str(binaries[version]),
                 "--samples", str(block["samples"]), "--cells", *block["cells"]],
                output / f"{name}-adapter",
            )
            run(
                [sys.executable, "scripts/slice135_e12_audit.py", "--receipt", str(output / name),
                 "--checkout", str(checkouts[version]), "--expectations", str(EXPECTED),
                 "--model-dir", str(MODEL), "--output", str(output / name / "audit.json")],
                output / f"{name}-audit",
            )
            entry["status"] = "audited"
        except Exception as error:
            entry["status"] = "invalid"
            entry["error"] = str(error)
            raise
        finally:
            entry["end_unix"] = time.time()
            schedule.write_text(json.dumps(manifest, indent=2) + "\n")
        if index + 1 < len(blocks):
            time.sleep(manifest["minimum_idle_seconds"])


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"INVALID_CAMPAIGN: {error}", file=sys.stderr, flush=True)
        raise SystemExit(1) from error
