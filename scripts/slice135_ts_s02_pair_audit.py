#!/usr/bin/env python3
"""Independently recompute paired installed-TypeScript S02 whole-sequence evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import tarfile

import slice135_python_s02_pair_audit as shared
import slice135_ts_s02_audit as ts_audit


ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
PAIR_ORDER = (
    ("baseline", "candidate"), ("candidate", "baseline"),
    ("baseline", "candidate"), ("candidate", "baseline"),
    ("baseline", "candidate"),
)


def sha(path: Path) -> str:
    """Hash exact retained bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def pair_delta_pct(baseline_ns: int, candidate_ns: int) -> float:
    """Compute the signed candidate-over-baseline paired latency delta."""
    if baseline_ns <= 0:
        raise ValueError("baseline latency must be positive")
    return 100 * (candidate_ns - baseline_ns) / baseline_ns


def validate_raw_binding(raw: dict, expected: dict) -> int:
    """Check exact source/artifact identity and the independent state oracle."""
    for key in (
        "source_sha", "runner_sha256", "s01_helper_sha256",
        "corpus_sha256", "graph_records_sha256",
    ):
        reference = "timed_runner_sha256" if key == "runner_sha256" else key
        if raw.get(key) != expected[reference]:
            raise ValueError(f"{key} source or runner identity changed")
    artifact = raw.get("artifact")
    if not isinstance(artifact, dict):
        raise ValueError("installed artifact missing")
    for key in ("module_sha256", "package_sha256", "native_sha256", "node_version"):
        if artifact.get(key) != expected[key]:
            raise ValueError(f"{key} artifact identity changed")
    return ts_audit.audit_raw(raw)


def archive_identity(main: Path, platform: Path, frozen: dict) -> dict[str, str]:
    """Recompute native and JS/package identities from retained npm archives."""
    if sha(main) != frozen["main_archive_sha256"] or sha(platform) != frozen["platform_archive_sha256"]:
        raise ValueError("npm archive differs from frozen comparison")
    with tarfile.open(main, "r:gz") as package:
        module = package.extractfile("package/dist/index.js")
        metadata = package.extractfile("package/package.json")
        if module is None or metadata is None:
            raise ValueError("main npm archive member missing")
        module_hash = hashlib.sha256(module.read()).hexdigest()
        package_hash = hashlib.sha256(metadata.read()).hexdigest()
    with tarfile.open(platform, "r:gz") as package:
        native = package.extractfile("package/fathomdb.linux-x64-gnu.node")
        if native is None:
            raise ValueError("native npm archive member missing")
        native_hash = hashlib.sha256(native.read()).hexdigest()
    return {
        "module_sha256": module_hash,
        "package_sha256": package_hash,
        "native_sha256": native_hash,
    }


def audit_block(
    directory: Path, *, role: str, protocol: dict, protocol_sha: str,
    archives: dict[str, str],
) -> dict:
    """Validate all 21 raw sequences and recompute one block from them."""
    manifest_path = directory / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    frozen = protocol[role]
    if (
        manifest.get("schema_version") != 1
        or manifest.get("status") != "FROZEN_TS_S02_PAIRED"
        or manifest.get("role") != role
        or manifest.get("comparison_protocol_sha256") != protocol_sha
        or manifest.get("source_sha") != frozen["source_sha"]
        or manifest.get("warmups") != protocol["warmup_per_block"]
        or manifest.get("samples") != protocol["samples_per_block"]
        or manifest.get("node_version") != protocol["node_version"]
        or manifest.get("corpus_sha256") != protocol["corpus_sha256"]
        or manifest.get("graph_records_sha256") != protocol["graph_records_sha256"]
    ):
        raise ValueError(f"{directory.name} manifest differs from freeze")
    for key in ("timed_runner_sha256", "s01_helper_sha256", "block_runner_sha256", "pilot_helper_sha256"):
        manifest_key = "runner_sha256" if key == "timed_runner_sha256" else key
        if manifest.get(manifest_key) != protocol[key]:
            raise ValueError(f"{directory.name} {key} differs from freeze")
    if (
        sha(directory / "bundle/slice135_ts_s02.mjs") != protocol["timed_runner_sha256"]
        or sha(directory / "bundle/slice135_ts_s01.mjs") != protocol["s01_helper_sha256"]
    ):
        raise ValueError(f"{directory.name} retained runner changed")
    for key, value in archives.items():
        if manifest.get(key) != value:
            raise ValueError(f"{directory.name} {key} differs from npm archive")
    for key in ("main_archive_sha256", "platform_archive_sha256"):
        if manifest.get("artifacts", {}).get(key) != frozen[key]:
            raise ValueError(f"{directory.name} {key} differs from freeze")

    attempts_path = directory / "attempts.json"
    environment_path = directory / "environment.json"
    summary = json.loads((directory / "summary.json").read_text())
    attempts = json.loads(attempts_path.read_text())
    labels = ["warmup"] + [f"sample-{index:03d}" for index in range(1, 21)]
    if [item.get("label") for item in attempts] != labels:
        raise ValueError(f"{directory.name} attempt order or count changed")
    expected = {
        "source_sha": frozen["source_sha"],
        "timed_runner_sha256": protocol["timed_runner_sha256"],
        "s01_helper_sha256": protocol["s01_helper_sha256"],
        "corpus_sha256": protocol["corpus_sha256"],
        "graph_records_sha256": protocol["graph_records_sha256"],
        "node_version": protocol["node_version"],
        **archives,
    }
    samples = []
    resources = []
    hashes = {}
    for attempt in attempts:
        label = attempt["label"]
        path = directory / f"{label}.json"
        raw = json.loads(path.read_text())
        duration = validate_raw_binding(raw, expected)
        if (
            attempt.get("valid") is not True
            or attempt.get("invalidators") != []
            or attempt.get("whole_product_ns") != duration
            or attempt.get("raw_sha256") != sha(path)
        ):
            raise ValueError(f"{directory.name} {label} attempt changed")
        command = json.loads((directory / f"{label}.command.json").read_text())
        ts_audit.check_command(command, directory.name, label)
        if (directory / f"{label}.stdout").stat().st_size or (
            directory / f"{label}.stderr"
        ).stat().st_size:
            raise ValueError(f"{directory.name} {label} emitted output")
        shared.check_resource(directory / f"{label}.resource.txt", attempt["resource"])
        resources.append(attempt["resource"])
        hashes[label] = sha(path)
        if label != "warmup":
            samples.append(duration)
    environment = json.loads(environment_path.read_text())
    warnings = shared.check_environment(environment, resources)
    p50 = shared.nearest_rank(samples, 0.50)
    p95 = shared.nearest_rank(samples, 0.95)
    if (
        summary.get("status") != "VALID_TS_S02_PAIRED_BLOCK"
        or summary.get("role") != role
        or summary.get("comparison_protocol_sha256") != protocol_sha
        or summary.get("invalidators") != []
        or summary.get("warnings") != sorted(set(warnings))
        or summary.get("valid_samples") != 20
        or summary.get("median_product_ns") != p50
        or summary.get("min_product_ns") != min(samples)
        or summary.get("max_product_ns") != max(samples)
        or summary.get("manifest_sha256") != sha(manifest_path)
        or summary.get("environment_sha256") != sha(environment_path)
        or summary.get("attempts_sha256") != sha(attempts_path)
    ):
        raise ValueError(f"{directory.name} summary changed")
    return {
        "role": role,
        "samples_ns": samples,
        "p50_ns": p50,
        "p95_ns": p95,
        "warnings": warnings,
        "max_peak_rss_kib": max(item["peak_rss_kib"] for item in resources),
        "total_user_cpu_s": sum(item["user_cpu_s"] for item in resources),
        "total_system_cpu_s": sum(item["system_cpu_s"] for item in resources),
        "raw_sha256": hashes,
    }


def audit_campaign(root: Path, protocol_path: Path, artifacts: dict) -> dict:
    """Recompute pooled and paired statistics from all retained valid blocks."""
    protocol = json.loads(protocol_path.read_text())
    protocol_sha = sha(protocol_path)
    if protocol.get("status") != "FROZEN_TS_S02_PAIRED" or protocol.get("pair_order") != [
        list(pair) for pair in PAIR_ORDER
    ]:
        raise ValueError("paired TypeScript S02 protocol changed")
    if sha(root / "freeze.json") != protocol_sha:
        raise ValueError("retained freeze bytes changed")
    if sha(root / "campaign.py") != protocol["campaign_runner_sha256"]:
        raise ValueError("campaign runner changed")
    if sha(root / "block-runner.py") != protocol["block_runner_sha256"]:
        raise ValueError("block runner changed")
    if sha(root / "workload.mjs") != protocol["timed_runner_sha256"]:
        raise ValueError("timed runner changed")
    if (
        sha(Path(__file__)) != protocol["paired_auditor_sha256"]
        or sha(root / "paired-auditor.py") != protocol["paired_auditor_sha256"]
    ):
        raise ValueError("paired auditor changed")
    manifest = json.loads((root / "run-manifest.json").read_text())
    if (
        manifest.get("status") != "FULL_CAMPAIGN_COMPLETE"
        or manifest.get("last_complete_pair") != 5
        or manifest.get("protocol_sha256") != protocol_sha
        or manifest.get("campaign_runner_sha256") != protocol["campaign_runner_sha256"]
    ):
        raise ValueError("paired campaign incomplete or source changed")
    archive_identities = {
        role: archive_identity(
            artifacts[role]["main"], artifacts[role]["platform"], protocol[role]
        ) for role in ("baseline", "candidate")
    }
    for role in ("baseline", "candidate"):
        checkout = artifacts[role]["checkout"]
        head = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=checkout,
            capture_output=True, text=True, check=True,
        ).stdout.strip()
        status = subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=all"],
            cwd=checkout, capture_output=True, text=True, check=True,
        ).stdout.strip()
        if head != protocol[role]["source_sha"] or status:
            raise ValueError(f"{role} measured source checkout changed")
    order = [json.loads(line) for line in (root / "run-order.jsonl").read_text().splitlines()]
    planned = [
        (pair, position, role)
        for pair, roles in enumerate(PAIR_ORDER, 1)
        for position, role in enumerate(roles, 1)
    ]
    if [(item.get("pair"), item.get("position"), item.get("role")) for item in order] != planned:
        raise ValueError("paired TypeScript S02 execution order changed")
    blocks = {}
    warnings_by_pair = {pair: [] for pair in range(1, 6)}
    for entry in order:
        pair, role = entry["pair"], entry["role"]
        name = f"pair-{pair:02d}-{role}"
        directory = root / name
        if (
            entry.get("name") != name
            or entry.get("exit_code") != 0
            or entry.get("stdout", "").strip() != "VALID_TS_S02_PAIRED_BLOCK"
            or entry.get("protocol_sha256") != protocol_sha
            or entry.get("summary_sha256") != sha(directory / "summary.json")
        ):
            raise ValueError(f"{name} campaign order receipt changed")
        result = audit_block(
            directory, role=role, protocol=protocol,
            protocol_sha=protocol_sha, archives=archive_identities[role],
        )
        blocks[name] = result
        warnings_by_pair[pair].extend(result["warnings"])
    pooled = {}
    for role in ("baseline", "candidate"):
        samples = [
            sample for pair in range(1, 6)
            for sample in blocks[f"pair-{pair:02d}-{role}"]["samples_ns"]
        ]
        if len(samples) != 100:
            raise ValueError(f"{role} sample count changed")
        pooled[role] = {
            "count": len(samples),
            "p50_ns": shared.nearest_rank(samples, 0.50),
            "p95_ns": shared.nearest_rank(samples, 0.95),
            "min_ns": min(samples), "max_ns": max(samples),
        }
    pairs = []
    for pair in range(1, 6):
        baseline = blocks[f"pair-{pair:02d}-baseline"]["p50_ns"]
        candidate = blocks[f"pair-{pair:02d}-candidate"]["p50_ns"]
        pairs.append({
            "pair": pair,
            "baseline_p50_ns": baseline,
            "candidate_p50_ns": candidate,
            "p50_delta_pct": pair_delta_pct(baseline, candidate),
            "warnings": sorted(set(warnings_by_pair[pair])),
        })
    warning_free = [item["p50_delta_pct"] for item in pairs if not item["warnings"]]
    deltas = [item["p50_delta_pct"] for item in pairs]
    return {
        "schema_version": 1,
        "status": "AUDITED_TS_S02_PAIRED_DIAGNOSTIC",
        "protocol_sha256": protocol_sha,
        "auditor_sha256": sha(Path(__file__)),
        "blocks": blocks,
        "pooled": pooled,
        "pooled_p50_delta_pct": pair_delta_pct(pooled["baseline"]["p50_ns"], pooled["candidate"]["p50_ns"]),
        "pooled_p95_delta_pct": pair_delta_pct(pooled["baseline"]["p95_ns"], pooled["candidate"]["p95_ns"]),
        "pairs": pairs,
        "median_pair_p50_delta_pct": statistics.median(deltas),
        "pair_p50_delta_range_pct": [min(deltas), max(deltas)],
        "warning_free_pairs": len(warning_free),
        "warning_free_median_pair_p50_delta_pct": statistics.median(warning_free)
        if len(warning_free) >= 3 else None,
        "interpretation": "Descriptive paired 0.8.26 versus exact-candidate installed TypeScript S02 latency; no equivalence, significance or release verdict.",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    for role in ("baseline", "candidate"):
        parser.add_argument(f"--{role}-checkout", required=True, type=Path)
        parser.add_argument(f"--{role}-main", required=True, type=Path)
        parser.add_argument(f"--{role}-platform", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("audit output already exists")
    artifacts = {
        role: {
            "checkout": getattr(args, f"{role}_checkout"),
            "main": getattr(args, f"{role}_main"),
            "platform": getattr(args, f"{role}_platform"),
        } for role in ("baseline", "candidate")
    }
    result = audit_campaign(args.root, args.protocol, artifacts)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"], flush=True)


if __name__ == "__main__":
    main()
