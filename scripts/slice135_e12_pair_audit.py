#!/usr/bin/env python3
"""Recompute frozen E01–E12 paired engine results from retained raw attempts."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

from slice135_e12_audit import _artifact_hash, audit_receipt, audit_resource_report


def digest(path: Path) -> str:
    """Hash the exact bytes of an evidence file."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_schedule(schedule: dict, frozen: dict) -> list[dict]:
    """Reject omitted, reordered or too-close blocks against the frozen pairs."""
    workload = frozen["workload"]
    minimum_idle = workload["minimum_idle_seconds_between_blocks"]
    if schedule.get("minimum_idle_seconds") != minimum_idle:
        raise ValueError("idle interval differs from freeze")
    campaign_hash = frozen.get("runner_sources", {}).get("campaign_sha256")
    if campaign_hash is not None and schedule.get("campaign_runner_sha256") != campaign_hash:
        raise ValueError("campaign runner differs from freeze")
    expected = []
    for group in ("query", "lifecycle"):
        for pair, order in enumerate(workload["pair_order"], start=1):
            if sorted(order) != ["baseline", "candidate"]:
                raise ValueError("frozen pair lacks both versions")
            for position, version in enumerate(order, start=1):
                expected.append((group, pair, position, version,
                                 f"{group}-pair-{pair}-{position}-{version}"))
    blocks = schedule.get("blocks")
    if not isinstance(blocks, list) or len(blocks) != len(expected):
        raise ValueError("campaign block count differs from freeze")
    previous_end = None
    for block, identity in zip(blocks, expected, strict=True):
        if not isinstance(block, dict):
            raise ValueError("malformed schedule block")
        if tuple(block.get(key) for key in ("group", "pair", "position", "version", "name")) != identity:
            raise ValueError("campaign block order differs from freeze")
        if block.get("status") != "audited":
            raise ValueError("campaign includes incomplete or invalid block")
        start, end = block.get("start_unix"), block.get("end_unix")
        if isinstance(start, bool) or isinstance(end, bool) or not isinstance(start, (int, float)) or not isinstance(end, (int, float)) or start <= 0 or end < start:
            raise ValueError("campaign block time malformed")
        if previous_end is not None and start - previous_end < minimum_idle:
            raise ValueError("campaign idle interval too short")
        previous_end = end
    return blocks


def paired_statistics(blocks: list[dict], summaries: dict[str, dict], frozen: dict) -> dict:
    """Derive per-pair percentiles and descriptive deltas without producer summaries."""
    workload = frozen["workload"]
    result = {}
    for group, cells, samples, statistics_names in (
        ("query", workload["query_cells"], workload["query_samples_per_block"], ("p50", "p95", "p99")),
        ("lifecycle", workload["lifecycle_cells"], workload["lifecycle_samples_per_block"], ("p50", "p95")),
    ):
        for cell in cells:
            result[cell] = {}
            for statistic in statistics_names:
                pairs = []
                for pair in range(1, len(workload["pair_order"]) + 1):
                    entries = [block for block in blocks if block["group"] == group and block["pair"] == pair]
                    if len(entries) != 2 or {entry["version"] for entry in entries} != {"baseline", "candidate"}:
                        raise ValueError(f"{group} pair {pair} incomplete")
                    observations = {}
                    for entry in entries:
                        measured = summaries.get(entry["name"], {}).get(cell)
                        if not isinstance(measured, dict) or measured.get("valid_samples") != samples:
                            raise ValueError(f"{entry['name']} {cell}: missing valid samples")
                        value = measured.get(f"{statistic}_ns")
                        if type(value) is not int or value <= 0:
                            raise ValueError(f"{entry['name']} {cell}: {statistic} unsupported")
                        observations[entry["version"]] = value
                    base, candidate = observations["baseline"], observations["candidate"]
                    pairs.append({"pair": pair, "baseline_ns": base, "candidate_ns": candidate,
                                  "delta_pct": 100 * (candidate - base) / base})
                deltas = [item["delta_pct"] for item in pairs]
                result[cell][statistic] = {
                    "pairs": pairs,
                    "pair_deltas_pct": deltas,
                    "median_delta_pct": statistics.median(deltas),
                    "delta_range_pct": [min(deltas), max(deltas)],
                }
    return result


def git(checkout: Path, *args: str) -> str:
    """Read a Git object or diff from an identified checkout."""
    return subprocess.check_output(["git", *args], cwd=checkout, text=True).strip()


def audit_campaign(campaign: Path, frozen_path: Path, expectations_path: Path,
                   model_dir: Path, baseline_checkout: Path, candidate_checkout: Path) -> dict:
    """Validate every artifact, state, order and sample before aggregating."""
    frozen = json.loads(frozen_path.read_text())
    if frozen.get("status") != "FROZEN_E01_E12_PAIRED_SUBSET":
        raise ValueError("E01–E12 subset not frozen")
    schedule = json.loads((campaign / "schedule.json").read_text())
    if schedule.get("freeze_sha256") != digest(frozen_path):
        raise ValueError("frozen protocol bytes mismatch")
    sources = {"baseline": schedule.get("baseline_source_sha"),
               "candidate": schedule.get("candidate_source_sha")}
    if sources["baseline"] != frozen["baseline"]["source_sha"]:
        raise ValueError("baseline source differs from freeze")
    if git(baseline_checkout, "rev-parse", "HEAD") != sources["baseline"]:
        raise ValueError("baseline checkout differs from receipt")
    if git(candidate_checkout, "rev-parse", f"{sources['candidate']}:src/rust/crates") != frozen["candidate_product_snapshot"]["rust_crates_git_tree"]:
        raise ValueError("measured candidate engine tree differs from freeze")
    if git(candidate_checkout, "rev-parse", f"{sources['candidate']}:Cargo.lock") != frozen["candidate_product_snapshot"]["cargo_lock_git_blob"]:
        raise ValueError("candidate lock differs from freeze")
    expected_checks = json.loads(expectations_path.read_text())
    if digest(expectations_path) != frozen["runner_sources"]["expected_checks_sha256"]:
        raise ValueError("expected checks changed")
    for name, source in (("adapter", "scripts/slice135_e12_adapter.py"),
                         ("workload", "scripts/slice135_e12_workload.rs")):
        measured_bytes = subprocess.check_output(
            ["git", "show", f"{sources['candidate']}:{source}"], cwd=candidate_checkout
        )
        if hashlib.sha256(measured_bytes).hexdigest() != frozen["runner_sources"][f"{name}_sha256"]:
            raise ValueError(f"{name} source changed")
    blocks = validate_schedule(schedule, frozen)
    summaries = {}
    resources = {}
    warnings = {}
    for block in blocks:
        root = campaign / block["name"]
        raw_bytes = (root / "raw.json").read_bytes()
        raw = json.loads(raw_bytes)
        protocol = json.loads((root / "protocol.json").read_text())
        version, group = block["version"], block["group"]
        if raw.get("source_sha") != sources[version] or protocol.get("source_sha") != sources[version]:
            raise ValueError(f"{block['name']}: wrong source")
        expected_cells = frozen["workload"][f"{group}_cells"]
        expected_samples = frozen["workload"][f"{group}_samples_per_block"]
        if protocol.get("settings", {}).get("samples") != expected_samples or set(protocol.get("cells", {})) != set(expected_cells):
            raise ValueError(f"{block['name']}: cells or sample rule changed")
        runner = json.loads((root / "runner").read_text())
        if runner != {"adapter": frozen["runner_sources"]["adapter_sha256"],
                       "workload": frozen["runner_sources"]["workload_sha256"]}:
            raise ValueError(f"{block['name']}: runner source changed")
        for cell in expected_cells:
            if protocol["cells"][cell]["expected_checks"] != expected_checks[cell]:
                raise ValueError(f"{block['name']} {cell}: expected output changed")
        summary = audit_receipt(raw, protocol, root, model_dir)
        retained = json.loads((root / "audit.json").read_text())
        if retained.get("cells") != summary["cells"] or retained.get("raw_sha256") != hashlib.sha256(raw_bytes).hexdigest():
            raise ValueError(f"{block['name']}: retained independent audit differs")
        gnu_time = (root / "run.stderr.log").read_text()
        resource = audit_resource_report(raw["resources"], gnu_time)
        if retained.get("resources") != resource:
            raise ValueError(f"{block['name']}: resource audit differs")
        provenance = json.loads((root / "build-provenance.json").read_text())
        if provenance.get("source_sha") != sources[version] or provenance.get("binary_sha256") != _artifact_hash(root / "binary"):
            raise ValueError(f"{block['name']}: binary provenance differs")
        summaries[block["name"]] = summary["cells"]
        resources[block["name"]] = resource
        warnings[block["name"]] = summary["environment_warnings"]
    return {
        "status": "PAIRED_DIAGNOSTIC",
        "freeze_sha256": digest(frozen_path),
        "schedule_sha256": digest(campaign / "schedule.json"),
        "sources": sources,
        "audited_blocks": len(blocks),
        "invalid_blocks": 0,
        "cells": paired_statistics(blocks, summaries, frozen),
        "resources_by_block": resources,
        "environment_warnings_by_block": warnings,
    }


def main() -> None:
    """Write a reproducible independent paired summary."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--campaign", required=True, type=Path)
    parser.add_argument("--freeze", required=True, type=Path)
    parser.add_argument("--expectations", required=True, type=Path)
    parser.add_argument("--model-dir", required=True, type=Path)
    parser.add_argument("--baseline-checkout", required=True, type=Path)
    parser.add_argument("--candidate-checkout", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    summary = audit_campaign(args.campaign, args.freeze, args.expectations,
                             args.model_dir, args.baseline_checkout, args.candidate_checkout)
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"status": summary["status"], "blocks": summary["audited_blocks"],
                      "cells": len(summary["cells"])}))


if __name__ == "__main__":
    main()
