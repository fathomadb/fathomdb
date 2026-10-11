#!/usr/bin/env python3
"""Capture one environment-checked installed-TypeScript S02 whole-sequence block."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

import slice135_ts_s01_block as s01_block


ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/slice135_ts_s02.mjs"
S01_HELPER = ROOT / "scripts/slice135_ts_s01.mjs"
PILOT = s01_block.PILOT
MIN_DISK_FREE_BYTES = 1_073_741_824
CORPUS_SHA256 = "e01c7b7772a925ab9c3a80ffb1d20ae1f9338c4fb0501fa16fcc844b871338dd"
GRAPH_RECORDS_SHA256 = "4e1ba86fc32c6b18fcd5b2d0c41e68add3d1d081677731b43d07e945a47d85bf"
EXPECTED_COUNTS = {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32}
SOURCE_BODY = '{"summary": "s02 canonical source evidence bytes"}'
STAGES = frozenset(
    (
        "open", "write", "configure_projection", "drain", "text", "vector",
        "hybrid", "hybrid_lexical_control", "evidence_search", "evidence_resolve",
        "graph_expand", "graph_target_resolve", "graph_edge_resolve", "erase",
        "erase_again", "close", "reopen", "reopened_close",
    )
)


def sha(path: Path) -> str:
    """Hash exact on-disk bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nearest_rank_median(samples: list[int]) -> int:
    """Return the lower middle observation for an even number of samples."""
    if not samples:
        raise ValueError("median requires samples")
    return sorted(samples)[(len(samples) - 1) // 2]


def validate_sample(raw: dict, manifest: dict) -> int:
    """Reject changed artifacts, included SQLite checks and false reopened state."""
    if (
        raw.get("schema_version") != 1
        or raw.get("status") != "UNFROZEN_TS_S02_PRODUCT_TIMING_FEASIBILITY"
        or raw.get("source_sha") != manifest["source_sha"]
    ):
        raise ValueError("source or timed-runner identity changed")
    if (
        raw.get("runner_sha256") != manifest["runner_sha256"]
        or raw.get("s01_helper_sha256") != manifest["s01_helper_sha256"]
    ):
        raise ValueError("runner or helper identity changed")
    artifact = raw.get("artifact")
    if not isinstance(artifact, dict):
        raise ValueError("installed artifact missing")
    for name in ("native_sha256", "module_sha256", "package_sha256"):
        if artifact.get(name) != manifest[name]:
            raise ValueError(f"{name} installed artifact changed")
    if artifact.get("node_version") != manifest["node_version"]:
        raise ValueError("Node version changed")
    if (
        raw.get("corpus_sha256") != manifest["corpus_sha256"]
        or raw.get("graph_records_sha256") != manifest["graph_records_sha256"]
    ):
        raise ValueError("corpus or graph fixture changed")
    if raw.get("semantic_ok") is not True:
        raise ValueError("producer semantic check failed")
    elapsed = raw.get("whole_product_ns")
    verification = raw.get("verification_ns")
    stages = raw.get("stage_ns")
    if (
        type(elapsed) is not int or elapsed <= 0
        or type(verification) is not int or verification <= 0
        or not isinstance(stages, dict) or set(stages) != STAGES
        or any(type(value) is not int or value <= 0 for value in stages.values())
    ):
        raise ValueError("product, verification or stage duration invalid")
    if elapsed < sum(stages.values()):
        raise ValueError("whole product timer shorter than stage sum")
    if raw.get("final_canonical_counts") != EXPECTED_COUNTS:
        raise ValueError("final canonical persistence changed")
    observed = raw.get("observed")
    if not isinstance(observed, dict):
        raise ValueError("materialized observation missing")
    if (
        observed.get("embedder") != "fathomdb-bge-small-en-v1.5"
        or observed.get("readiness") != "ready"
        or observed.get("unsupported_kinds") != []
        or observed.get("anchor_before") is not True
    ):
        raise ValueError("model, projection or initial state changed")
    if observed.get("text") != {"ids": ["A"], "branches": ["text"]}:
        raise ValueError("text result changed")
    if (
        not observed.get("vector", {}).get("ids")
        or "vector" not in observed["vector"].get("branches", [])
        or "A" not in observed.get("hybrid", {}).get("ids", [])
        or "A" not in observed.get("hybrid_lexical_ids", [])
    ):
        raise ValueError("vector or hybrid result changed")
    if observed.get("evidence") != {
        "logical_id": "s02-claim", "source_body": SOURCE_BODY,
    } or observed.get("graph") != {
        "target_id": "s02-claim", "target_revision": "s02-claim-r1",
        "edge_revision": "s02-edge-r1", "edge_from": "s02-root",
        "edge_to": "s02-claim", "source_body": SOURCE_BODY,
    }:
        raise ValueError("evidence or graph result changed")
    if observed.get("erasure") != {
        "source_ref": "slice135-s02-graph", "nodes_excised": 3, "edges_excised": 1,
    } or observed.get("second_erasure") != {"nodes_excised": 0, "edges_excised": 0}:
        raise ValueError("erasure result changed")
    for name in ("after_erasure", "after_reopen"):
        state = observed.get(name)
        if not isinstance(state, dict):
            raise ValueError(f"{name} state missing")
        if "canonical_counts" in state:
            raise ValueError("interleaved SQLite verification entered product timer")
        for field in (
            "source_absent", "root_absent", "claim_absent", "anchor_retained",
            "evidence_query_empty", "graph_empty",
        ):
            if state.get(field) is not True:
                if name == "after_reopen" and field == "source_absent":
                    raise ValueError("reopened source remains")
                raise ValueError(f"{name} {field} failed")
    if observed["after_reopen"].get("readiness") != "ready":
        raise ValueError("reopened projection not ready")
    return elapsed


def validate_comparison_protocol(
    specification: dict,
    *,
    role: str,
    source_sha: str,
    main_sha256: str,
    platform_sha256: str,
    samples: int,
) -> None:
    """Bind a paired block to a frozen TypeScript S02 subset."""
    if (
        specification.get("schema_version") != 1
        or specification.get("status") != "FROZEN_TS_S02_PAIRED"
    ):
        raise ValueError("S02 TypeScript paired protocol is not frozen")
    if role not in ("baseline", "candidate") or specification.get(role) != {
        "source_sha": source_sha,
        "main_archive_sha256": main_sha256,
        "platform_archive_sha256": platform_sha256,
    }:
        raise ValueError(f"{role} source or artifact differs from protocol")
    if specification.get("samples_per_block") != samples:
        raise ValueError("paired sample count differs from protocol")
    for name, path in (
        ("timed_runner_sha256", RUNNER),
        ("s01_helper_sha256", S01_HELPER),
        ("block_runner_sha256", Path(__file__)),
        ("pilot_helper_sha256", ROOT / "scripts/slice135_pilot.py"),
    ):
        if specification.get(name) != sha(path):
            raise ValueError(f"{name} differs from protocol")
    if (
        specification.get("corpus_sha256") != CORPUS_SHA256
        or specification.get("graph_records_sha256") != GRAPH_RECORDS_SHA256
        or specification.get("node_version") != "v25.9.0"
    ):
        raise ValueError("fixture or Node version differs from protocol")


def run_block(
    *,
    checkout: Path,
    source_sha: str,
    main_archive: Path,
    platform_archive: Path,
    main_sha256: str,
    platform_sha256: str,
    install_root: Path,
    node: Path,
    samples: int,
    output: Path,
    comparison_protocol: Path | None = None,
    role: str = "baseline",
) -> dict:
    """Retain warm-up, every whole-sequence attempt and host/child resources."""
    if samples < 3:
        raise ValueError("S02 block requires at least three measured sequences")
    if not PILOT.GNU_TIME.is_file():
        raise ValueError("GNU Time unavailable")
    source = s01_block.verify_source(checkout, source_sha)
    artifacts = s01_block.verify_archives(
        main_archive, platform_archive, install_root, main_sha256, platform_sha256
    )
    node_version = subprocess.run(
        [str(node), "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    if node_version != "v25.9.0":
        raise ValueError("Node version differs from qualified S02 runtime")
    comparison_sha256 = None
    if comparison_protocol is not None:
        validate_comparison_protocol(
            json.loads(comparison_protocol.read_text()), role=role,
            source_sha=source_sha, main_sha256=main_sha256,
            platform_sha256=platform_sha256, samples=samples,
        )
        comparison_sha256 = sha(comparison_protocol)
    elif role != "baseline":
        raise ValueError("candidate S02 block requires frozen comparison protocol")
    output.mkdir(parents=True, exist_ok=False)
    bundle = output / "bundle"
    bundle.mkdir()
    for path in (RUNNER, S01_HELPER):
        shutil.copyfile(path, bundle / path.name)
    manifest = {
        "schema_version": 1,
        "status": "FROZEN_TS_S02_PAIRED" if comparison_sha256 else "BASELINE_TS_S02_NOISE_BLOCK",
        "role": role,
        "comparison_protocol_sha256": comparison_sha256,
        "source_sha": source_sha,
        "source": source,
        "artifacts": artifacts,
        "runner_sha256": sha(bundle / RUNNER.name),
        "s01_helper_sha256": sha(bundle / S01_HELPER.name),
        "block_runner_sha256": sha(Path(__file__)),
        "pilot_helper_sha256": sha(ROOT / "scripts/slice135_pilot.py"),
        "native_sha256": artifacts["native_sha256"],
        "module_sha256": artifacts["module_sha256"],
        "package_sha256": sha(install_root / "node_modules/fathomdb/package.json"),
        "corpus_sha256": CORPUS_SHA256,
        "graph_records_sha256": GRAPH_RECORDS_SHA256,
        "node_version": node_version,
        "warmups": 1,
        "samples": samples,
        "timing_boundary": "installed TypeScript SDK open through reopened close; direct SQLite verification excluded",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    environment = {**os.environ, "FATHOMDB_EMBED_DEVICE": "cpu", "HF_HUB_OFFLINE": "1"}
    start = PILOT.inventory(output)
    start["node_version"] = node_version
    attempts = []
    for number in range(samples + 1):
        label = "warmup" if number == 0 else f"sample-{number:03d}"
        raw_path = output / f"{label}.json"
        resource_path = output / f"{label}.resource.txt"
        command = [
            str(PILOT.GNU_TIME), "-o", str(resource_path), "-f", PILOT.RESOURCE_FORMAT,
            str(node), str(bundle / RUNNER.name),
            "--install-root", str(install_root),
            "--source-sha", source_sha,
            "--expected-native-sha256", artifacts["native_sha256"],
            "--timing-mode", "product", "--output", str(raw_path),
        ]
        (output / f"{label}.command.json").write_text(json.dumps(command) + "\n")
        invalid: list[str] = []
        try:
            result = subprocess.run(
                command, cwd=ROOT, capture_output=True, text=True,
                env=environment, check=False, timeout=180,
            )
            (output / f"{label}.stdout").write_text(result.stdout)
            (output / f"{label}.stderr").write_text(result.stderr)
            if result.returncode:
                invalid.append(f"process exited {result.returncode}")
        except subprocess.TimeoutExpired as error:
            (output / f"{label}.stdout").write_bytes(error.stdout or b"")
            (output / f"{label}.stderr").write_bytes(error.stderr or b"")
            invalid.append("process timed out after 180 seconds")
        resource = PILOT.read_resource_report(resource_path)
        if not PILOT.complete_child_resources(resource) or resource["swap_events"] > 0:
            invalid.append("child resource report missing or swapped")
        try:
            raw = json.loads(raw_path.read_text())
            elapsed = validate_sample(raw, manifest)
        except (OSError, ValueError, KeyError, TypeError) as error:
            invalid.append(f"raw validation failed: {error}")
            elapsed = None
        attempts.append({
            "label": label, "valid": not invalid, "invalidators": invalid,
            "whole_product_ns": elapsed,
            "raw_sha256": sha(raw_path) if raw_path.is_file() else None,
            "resource": resource,
        })
    end = PILOT.inventory(output)
    end["node_version"] = node_version
    environment_invalid = sorted(set(
        reason
        for attempt in attempts
        for reason in PILOT.environment_invalidators(
            start, end, MIN_DISK_FREE_BYTES, attempt["resource"]
        )
    ))
    warnings = sorted(set(
        reason
        for attempt in attempts
        for reason in PILOT.environment_warnings(start, end, attempt["resource"])
    ))
    (output / "environment.json").write_text(json.dumps({
        "start": start, "end": end,
        "invalidators": environment_invalid, "warnings": warnings,
    }, indent=2) + "\n")
    (output / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
    measured = [attempt["whole_product_ns"] for attempt in attempts[1:]]
    invalid = environment_invalid + [
        f"{attempt['label']}: {reason}"
        for attempt in attempts for reason in attempt["invalidators"]
    ]
    status = "INVALID_BLOCK" if invalid else (
        "VALID_TS_S02_PAIRED_BLOCK" if comparison_sha256 else "VALID_BASELINE_TS_S02_PILOT_BLOCK"
    )
    summary = {
        "schema_version": 1, "status": status, "role": role,
        "comparison_protocol_sha256": comparison_sha256,
        "invalidators": invalid, "warnings": warnings,
        "manifest_sha256": sha(output / "manifest.json"),
        "environment_sha256": sha(output / "environment.json"),
        "attempts_sha256": sha(output / "attempts.json"),
        "valid_samples": len(measured) if not invalid else 0,
        "median_product_ns": nearest_rank_median(measured) if not invalid else None,
        "min_product_ns": min(measured) if not invalid else None,
        "max_product_ns": max(measured) if not invalid else None,
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--main-archive", required=True, type=Path)
    parser.add_argument("--platform-archive", required=True, type=Path)
    parser.add_argument("--main-sha256", required=True)
    parser.add_argument("--platform-sha256", required=True)
    parser.add_argument("--install-root", required=True, type=Path)
    parser.add_argument("--node", required=True, type=Path)
    parser.add_argument("--samples", required=True, type=int)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--comparison-protocol", type=Path)
    parser.add_argument("--role", choices=("baseline", "candidate"), default="baseline")
    args = parser.parse_args()
    summary = run_block(
        checkout=args.checkout.resolve(), source_sha=args.source_sha,
        main_archive=args.main_archive.resolve(),
        platform_archive=args.platform_archive.resolve(),
        main_sha256=args.main_sha256, platform_sha256=args.platform_sha256,
        install_root=args.install_root.resolve(), node=args.node.resolve(),
        samples=args.samples, output=args.output_dir.absolute(),
        comparison_protocol=args.comparison_protocol, role=args.role,
    )
    print(summary["status"], flush=True)
    return 0 if summary["status"].startswith("VALID_") else 1


if __name__ == "__main__":
    raise SystemExit(main())
