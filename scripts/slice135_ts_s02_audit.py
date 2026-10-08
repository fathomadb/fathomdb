#!/usr/bin/env python3
"""Independently audit installed-TypeScript S02 baseline whole-sequence blocks."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile

import slice135_python_s02_pair_audit as shared


ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/slice135_ts_s02.mjs"
S01_HELPER = ROOT / "scripts/slice135_ts_s01.mjs"
BLOCK_RUNNER = ROOT / "scripts/slice135_ts_s02_block.py"
PILOT_HELPER = ROOT / "scripts/slice135_pilot.py"
BASELINE_SOURCE = "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
BASELINE_MAIN = "90363762405041b11e6dd654da9cb6347695399eb37b81c5c09fde91296ac336"
BASELINE_PLATFORM = "b59b1862b11b8ed3edcdc2e5ee86f9db142268bb5e14571054f6245718fc28f6"
CORPUS_SHA256 = "e01c7b7772a925ab9c3a80ffb1d20ae1f9338c4fb0501fa16fcc844b871338dd"
GRAPH_RECORDS_SHA256 = "4e1ba86fc32c6b18fcd5b2d0c41e68add3d1d081677731b43d07e945a47d85bf"


def sha(path: Path) -> str:
    """Hash the exact retained bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit_raw(raw: dict) -> int:
    """Check a product-only timer and independently specified real-DB state."""
    if (
        raw.get("schema_version") != 1
        or raw.get("status") != "UNFROZEN_TS_S02_PRODUCT_TIMING_FEASIBILITY"
    ):
        raise ValueError("timed TypeScript S02 schema or mode changed")
    observed = raw.get("observed")
    if not isinstance(observed, dict):
        raise ValueError("materialized observations missing")
    for phase in ("after_erasure", "after_reopen"):
        if "canonical_counts" in observed.get(phase, {}):
            raise ValueError("interleaved SQLite validation entered product timer")
    shared.validate_materialized(raw)
    stages = raw.get("stage_ns")
    elapsed = raw.get("whole_product_ns")
    verification = raw.get("verification_ns")
    if (
        not isinstance(stages, dict) or set(stages) != shared.STAGES
        or any(type(value) is not int or value <= 0 for value in stages.values())
        or type(elapsed) is not int or elapsed <= 0
        or type(verification) is not int or verification <= 0
    ):
        raise ValueError("S02 product or verification timer invalid")
    if elapsed < sum(stages.values()):
        raise ValueError("whole product timer shorter than stage sum")
    return elapsed


def archive_identity(main: Path, platform: Path) -> dict[str, str]:
    """Recompute installed module, package and native hashes from npm archives."""
    if sha(main) != BASELINE_MAIN or sha(platform) != BASELINE_PLATFORM:
        raise ValueError("baseline npm archive identity changed")
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


def check_command(command: list[str], block: str, label: str) -> None:
    """Keep the archived block/output identity without fixing its root path."""
    if (
        command.count("--timing-mode") != 1
        or command.count("--output") != 1
        or command[command.index("--timing-mode") + 1] != "product"
        or Path(command[command.index("--output") + 1]).parts[-2:]
        != (block, f"{label}.json")
    ):
        raise ValueError(f"{block} {label} command changed")


def audit_block(directory: Path, identities: dict[str, str]) -> dict:
    """Recompute every sample, resource check and host warning for one block."""
    manifest_path = directory / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    if (
        manifest.get("schema_version") != 1
        or manifest.get("status") != "BASELINE_TS_S02_NOISE_BLOCK"
        or manifest.get("role") != "baseline"
        or manifest.get("source_sha") != BASELINE_SOURCE
        or manifest.get("warmups") != 1
        or manifest.get("samples") != 3
        or manifest.get("corpus_sha256") != CORPUS_SHA256
        or manifest.get("graph_records_sha256") != GRAPH_RECORDS_SHA256
        or manifest.get("node_version") != "v25.9.0"
    ):
        raise ValueError(f"{directory.name} manifest changed")
    for key, source, retained in (
        ("runner_sha256", RUNNER, "slice135_ts_s02.mjs"),
        ("s01_helper_sha256", S01_HELPER, "slice135_ts_s01.mjs"),
    ):
        if manifest.get(key) != sha(source) or sha(directory / "bundle" / retained) != sha(source):
            raise ValueError(f"{directory.name} {key} changed")
    if (
        manifest.get("block_runner_sha256") != sha(BLOCK_RUNNER)
        or manifest.get("pilot_helper_sha256") != sha(PILOT_HELPER)
    ):
        raise ValueError(f"{directory.name} orchestration code changed")
    for key, value in identities.items():
        if manifest.get(key) != value:
            raise ValueError(f"{directory.name} {key} changed")
    for key, value in (("main_archive_sha256", BASELINE_MAIN),
                       ("platform_archive_sha256", BASELINE_PLATFORM)):
        if manifest.get("artifacts", {}).get(key) != value:
            raise ValueError(f"{directory.name} archive hash changed")

    attempts_path = directory / "attempts.json"
    environment_path = directory / "environment.json"
    summary = json.loads((directory / "summary.json").read_text())
    attempts = json.loads(attempts_path.read_text())
    labels = ["warmup", "sample-001", "sample-002", "sample-003"]
    if [attempt.get("label") for attempt in attempts] != labels:
        raise ValueError(f"{directory.name} attempt count or order changed")
    samples = []
    resources = []
    raw_hashes = {}
    for attempt in attempts:
        label = attempt["label"]
        path = directory / f"{label}.json"
        raw = json.loads(path.read_text())
        if (
            raw.get("source_sha") != BASELINE_SOURCE
            or raw.get("runner_sha256") != manifest["runner_sha256"]
            or raw.get("s01_helper_sha256") != manifest["s01_helper_sha256"]
            or raw.get("corpus_sha256") != CORPUS_SHA256
            or raw.get("graph_records_sha256") != GRAPH_RECORDS_SHA256
            or raw.get("artifact", {}).get("node_version") != "v25.9.0"
        ):
            raise ValueError(f"{directory.name} {label} source or runner changed")
        for key, value in identities.items():
            if raw["artifact"].get(key) != value:
                raise ValueError(f"{directory.name} {label} {key} changed")
        duration = audit_raw(raw)
        if (
            attempt.get("valid") is not True
            or attempt.get("invalidators") != []
            or attempt.get("whole_product_ns") != duration
            or attempt.get("raw_sha256") != sha(path)
        ):
            raise ValueError(f"{directory.name} {label} attempt receipt changed")
        command = json.loads((directory / f"{label}.command.json").read_text())
        check_command(command, directory.name, label)
        if (directory / f"{label}.stdout").stat().st_size or (
            directory / f"{label}.stderr"
        ).stat().st_size:
            raise ValueError(f"{directory.name} {label} emitted output")
        shared.check_resource(directory / f"{label}.resource.txt", attempt["resource"])
        resources.append(attempt["resource"])
        raw_hashes[label] = sha(path)
        if label != "warmup":
            samples.append(duration)
    environment = json.loads(environment_path.read_text())
    warnings = shared.check_environment(environment, resources)
    median = shared.nearest_rank(samples, 0.5)
    if (
        summary.get("status") != "VALID_BASELINE_TS_S02_PILOT_BLOCK"
        or summary.get("role") != "baseline"
        or summary.get("invalidators") != []
        or summary.get("warnings") != sorted(set(warnings))
        or summary.get("valid_samples") != 3
        or summary.get("median_product_ns") != median
        or summary.get("min_product_ns") != min(samples)
        or summary.get("max_product_ns") != max(samples)
        or summary.get("manifest_sha256") != sha(manifest_path)
        or summary.get("environment_sha256") != sha(environment_path)
        or summary.get("attempts_sha256") != sha(attempts_path)
    ):
        raise ValueError(f"{directory.name} summary changed")
    return {
        "block": directory.name,
        "samples_ns": samples,
        "median_ns": median,
        "warnings": warnings,
        "peak_rss_kib": [resource["peak_rss_kib"] for resource in resources],
        "raw_sha256": raw_hashes,
    }


def audit_pilot(root: Path, main: Path, platform: Path, checkout: Path) -> dict:
    """Audit five baseline-only blocks and report noise without a version claim."""
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=checkout, capture_output=True, text=True,
        check=True,
    ).stdout.strip()
    status = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=all"],
        cwd=checkout, capture_output=True, text=True, check=True,
    ).stdout.strip()
    if head != BASELINE_SOURCE or status:
        raise ValueError("baseline source checkout changed")
    driver = json.loads((root / "driver.json").read_text())
    if (
        driver.get("blocks") != 5
        or driver.get("samples_per_block") != 3
        or driver.get("warmups_per_block") != 1
        or driver.get("idle_seconds", 0) < 10
        or driver.get("block_runner_sha256") != sha(BLOCK_RUNNER)
        or driver.get("source_sha") != BASELINE_SOURCE
        or driver.get("main_sha256") != BASELINE_MAIN
        or driver.get("platform_sha256") != BASELINE_PLATFORM
    ):
        raise ValueError("baseline pilot driver changed")
    identities = archive_identity(main, platform)
    order = [json.loads(line) for line in (root / "run-order.jsonl").read_text().splitlines()]
    if [item.get("block") for item in order] != list(range(1, 6)):
        raise ValueError("baseline pilot block order changed")
    blocks = []
    for index, record in enumerate(order, 1):
        directory = root / f"block-{index:02d}"
        if record.get("status") != "VALID_BASELINE_TS_S02_PILOT_BLOCK" or (
            record.get("summary_sha256") != sha(directory / "summary.json")
        ):
            raise ValueError(f"block-{index:02d} run order changed")
        blocks.append(audit_block(directory, identities))
    medians = [item["median_ns"] for item in blocks]
    middle = sorted(medians)[2]
    return {
        "schema_version": 1,
        "status": "CONTROLLED_TS_S02_BASELINE_ONLY_NO_PAIR",
        "baseline_source_sha": BASELINE_SOURCE,
        "main_archive_sha256": BASELINE_MAIN,
        "platform_archive_sha256": BASELINE_PLATFORM,
        "runner_sha256": sha(RUNNER),
        "block_runner_sha256": sha(BLOCK_RUNNER),
        "auditor_sha256": sha(Path(__file__)),
        "blocks": blocks,
        "measured_sequences": sum(len(item["samples_ns"]) for item in blocks),
        "block_median_spread_ns": max(medians) - min(medians),
        "block_median_spread_pct_of_median": 100 * (max(medians) - min(medians)) / middle,
        "noise_note": "baseline-only; three samples per block cannot support p95 or p99",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--main-archive", required=True, type=Path)
    parser.add_argument("--platform-archive", required=True, type=Path)
    parser.add_argument("--baseline-checkout", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("audit output already exists")
    result = audit_pilot(
        args.root, args.main_archive, args.platform_archive,
        args.baseline_checkout,
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"], result["measured_sequences"], flush=True)


if __name__ == "__main__":
    main()
