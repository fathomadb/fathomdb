#!/usr/bin/env python3
"""Independently recompute installed-Python S02 paired whole-product receipts."""

from __future__ import annotations

import argparse
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
import statistics
import zipfile


STAGES = {
    "open",
    "write",
    "configure_projection",
    "drain",
    "text",
    "vector",
    "hybrid",
    "hybrid_lexical_control",
    "evidence_search",
    "evidence_resolve",
    "graph_expand",
    "graph_target_resolve",
    "graph_edge_resolve",
    "erase",
    "erase_again",
    "close",
    "reopen",
    "reopened_close",
}
STABLE = ("host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler")
COUNTS = {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32}
SOURCE_BODY = '{"summary": "s02 canonical source evidence bytes"}'
GRAPH = {
    "target_id": "s02-claim",
    "target_revision": "s02-claim-r1",
    "edge_revision": "s02-edge-r1",
    "edge_from": "s02-root",
    "edge_to": "s02-claim",
    "source_body": SOURCE_BODY,
}
RESOURCE_KEYS = {
    "user_s": "user_cpu_s",
    "system_s": "system_cpu_s",
    "peak_rss_kib": "peak_rss_kib",
    "fs_inputs": "fs_inputs",
    "fs_outputs": "fs_outputs",
    "major_faults": "major_faults",
    "swap_events": "swap_events",
}


def sha(path: Path) -> str:
    """Hash the exact retained bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nearest_rank(values: list[int], quantile: float) -> int:
    """Select a percentile without interpolation or trimming."""
    if not values or not 0 < quantile <= 1:
        raise ValueError("nearest-rank input invalid")
    return sorted(values)[math.ceil(quantile * len(values)) - 1]


def validate_materialized(raw: dict) -> None:
    """Check seeded facts, graph/evidence identity and reopened erasure state."""
    if raw["semantic_ok"] is not True:
        raise ValueError("producer semantic failure")
    if raw["final_canonical_counts"] != COUNTS:
        raise ValueError("reopened canonical rows changed")
    observed = raw["observed"]
    if observed["embedder"] != "fathomdb-bge-small-en-v1.5":
        raise ValueError("embedder changed")
    if observed["unsupported_kinds"] != [] or observed["readiness"] != "ready":
        raise ValueError("projection not ready")
    if observed["anchor_before"] is not True:
        raise ValueError("anchor missing before queries")
    if observed["text"] != {"ids": ["A"], "branches": ["text"]}:
        raise ValueError("text anchor changed")
    eligible = {"A", "B", "s02-source", "s02-root", "s02-claim"} | {
        f"F{index:04}" for index in range(30)
    }
    for name in ("text", "vector", "hybrid"):
        result = observed[name]
        if any(identifier not in eligible for identifier in result["ids"]):
            raise ValueError(f"{name} returned an ineligible id")
        if len(result["ids"]) != len(result["branches"]):
            raise ValueError(f"{name} result malformed")
    if not observed["vector"]["ids"] or "vector" not in observed["vector"]["branches"]:
        raise ValueError("vector branch missing")
    if (
        "A" not in observed["hybrid"]["ids"]
        or "A" not in observed["hybrid_lexical_ids"]
    ):
        raise ValueError("hybrid anchor missing")
    if any(
        branch not in ("text", "vector") for branch in observed["hybrid"]["branches"]
    ):
        raise ValueError("hybrid branch invalid")
    if observed["evidence"] != {"logical_id": "s02-claim", "source_body": SOURCE_BODY}:
        raise ValueError("source evidence changed")
    if observed["graph"] != GRAPH:
        raise ValueError("graph evidence changed")
    if observed["erasure"] != {
        "source_ref": "slice135-s02-graph",
        "nodes_excised": 3,
        "edges_excised": 1,
    } or observed["second_erasure"] != {"nodes_excised": 0, "edges_excised": 0}:
        raise ValueError("erasure report changed")
    for phase in ("after_erasure", "after_reopen"):
        for key in (
            "source_absent",
            "root_absent",
            "claim_absent",
            "anchor_retained",
            "evidence_query_empty",
            "graph_empty",
        ):
            if observed[phase][key] is not True:
                raise ValueError(f"{phase} {key} failed")
    if observed["after_reopen"]["readiness"] != "ready":
        raise ValueError("reopened projection not ready")


def check_resource(path: Path, recorded: dict) -> None:
    fields = dict(
        line.split("=", 1) for line in path.read_text().splitlines() if "=" in line
    )
    for field, key in RESOURCE_KEYS.items():
        value = (
            float(fields[field])
            if field in ("user_s", "system_s")
            else int(fields[field])
        )
        if recorded[key] != value:
            raise ValueError(f"GNU Time {key} changed")
    if recorded["method"] != "gnu-time" or recorded["unsupported"] != []:
        raise ValueError("child resource report incomplete")


def check_environment(environment: dict, resources: list[dict]) -> list[str]:
    start, end = environment["start"], environment["end"]
    if any(not start.get(key) or start[key] != end.get(key) for key in STABLE):
        raise ValueError("host identity drift")
    if (
        start["governor"] == "unavailable"
        or start["competing_jobs"]
        or end["competing_jobs"]
    ):
        raise ValueError("governor or competing jobs invalid")
    if min(start["disk_free_bytes"], end["disk_free_bytes"]) < 1_073_741_824:
        raise ValueError("disk pressure")
    if end["swap_pages"] < start["swap_pages"] or any(
        r["swap_events"] for r in resources
    ):
        raise ValueError("swap control invalid")
    if environment["invalidators"]:
        raise ValueError("producer environment invalidator retained")
    warnings = []
    delta = end["swap_pages"] - start["swap_pages"]
    if delta:
        warnings.append(f"host swap drift: {delta} pages; child swap events: 0")
    for resource in resources:
        if resource["major_faults"]:
            warnings.append(f"child major faults: {resource['major_faults']}")
    if sorted(set(warnings)) != sorted(environment["warnings"]):
        raise ValueError("paging warning differs from counters")
    return warnings


def audit_block(
    directory: Path,
    *,
    protocol: dict,
    protocol_sha256: str,
    role: str,
    wheel: Path,
) -> dict:
    """Check every raw attempt and recompute one declared paired block."""
    manifest_path = directory / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    if (
        manifest["schema_version"] != 1
        or manifest["status"] != "FROZEN_S02_PYTHON_PAIRED"
        or manifest["role"] != role
        or manifest["comparison_protocol_sha256"] != protocol_sha256
        or manifest["source_sha"] != protocol[role]["source_sha"]
        or manifest["wheel_sha256"] != protocol[role]["wheel_sha256"]
        or manifest["samples"] != protocol["samples_per_block"]
        or manifest["warmups"] != protocol["warmup_per_block"]
        or manifest["corpus_sha256"] != protocol["corpus_sha256"]
        or manifest["graph_records_sha256"] != protocol["graph_records_sha256"]
    ):
        raise ValueError(f"{directory.name} manifest differs from protocol")
    if sha(wheel) != protocol[role]["wheel_sha256"]:
        raise ValueError(f"{role} wheel bytes changed")
    with zipfile.ZipFile(wheel) as archive:
        native_sha = hashlib.sha256(
            archive.read("fathomdb/_fathomdb.abi3.so")
        ).hexdigest()
    if manifest["native_sha256"] != native_sha:
        raise ValueError(f"{role} native bytes changed")
    for key, name in (
        ("timed_runner_sha256", "slice135_python_s02_timing.py"),
        ("s01_helper_sha256", "slice135_python_s01.py"),
        ("s02_helper_sha256", "slice135_python_s02.py"),
        ("pilot_helper_sha256", "slice135_pilot.py"),
    ):
        if manifest[key.replace("timed_runner", "runner")] != protocol[key] or (
            sha(directory / "bundle" / name) != protocol[key]
        ):
            raise ValueError(f"{directory.name} {key} changed")
    environment_path = directory / "environment.json"
    attempts_path = directory / "attempts.json"
    summary = json.loads((directory / "summary.json").read_text())
    if (
        summary["status"] != "VALID_S02_PAIRED_BLOCK"
        or summary["role"] != role
        or summary["comparison_protocol_sha256"] != protocol_sha256
        or summary["invalidators"] != []
        or summary["manifest_sha256"] != sha(manifest_path)
        or summary["environment_sha256"] != sha(environment_path)
        or summary["attempts_sha256"] != sha(attempts_path)
    ):
        raise ValueError(f"{directory.name} summary binding changed")
    attempts = json.loads(attempts_path.read_text())
    labels = ["warmup"] + [f"sample-{index:03d}" for index in range(1, 21)]
    if [item["label"] for item in attempts] != labels:
        raise ValueError(f"{directory.name} attempt order changed")
    resources = []
    measured = []
    raw_hashes = {}
    first_raw = None
    for attempt in attempts:
        label = attempt["label"]
        path = directory / f"{label}.json"
        raw = json.loads(path.read_text())
        if (
            raw["source_sha"] != protocol[role]["source_sha"]
            or raw["artifact"]["wheel_sha256"] != protocol[role]["wheel_sha256"]
            or raw["artifact"]["native_sha256"] != native_sha
            or raw["runner_sha256"] != protocol["timed_runner_sha256"]
            or raw["s01_helper_sha256"] != protocol["s01_helper_sha256"]
            or raw["s02_helper_sha256"] != protocol["s02_helper_sha256"]
            or raw["corpus_sha256"] != protocol["corpus_sha256"]
            or raw["graph_records_sha256"] != protocol["graph_records_sha256"]
        ):
            raise ValueError(f"{directory.name} {label} raw identity changed")
        validate_materialized(raw)
        stages = raw["stage_ns"]
        if set(stages) != STAGES or any(
            type(v) is not int or v <= 0 for v in stages.values()
        ):
            raise ValueError(f"{directory.name} {label} stages invalid")
        if (
            raw["whole_product_ns"] < sum(stages.values())
            or raw["verification_ns"] <= 0
        ):
            raise ValueError(f"{directory.name} {label} timer invalid")
        if (
            not attempt["valid"]
            or attempt["invalidators"]
            or (
                attempt["raw_sha256"] != sha(path)
                or attempt["whole_product_ns"] != raw["whole_product_ns"]
            )
        ):
            raise ValueError(f"{directory.name} {label} attempt changed")
        check_resource(directory / f"{label}.resource.txt", attempt["resource"])
        if (directory / f"{label}.stdout").stat().st_size or (
            directory / f"{label}.stderr"
        ).stat().st_size:
            raise ValueError(f"{directory.name} {label} emitted output")
        resources.append(attempt["resource"])
        raw_hashes[str(path.relative_to(directory.parent))] = sha(path)
        if label != "warmup":
            measured.append(raw["whole_product_ns"])
            if first_raw is None:
                first_raw = raw
    environment = json.loads(environment_path.read_text())
    warnings = check_environment(environment, resources)
    if (
        summary["valid_samples"] != 20
        or summary["median_product_ns"] != nearest_rank(measured, 0.5)
        or summary["min_product_ns"] != min(measured)
        or summary["max_product_ns"] != max(measured)
    ):
        raise ValueError(f"{directory.name} summary statistic changed")
    return {
        "role": role,
        "samples_ns": measured,
        "p50_ns": nearest_rank(measured, 0.5),
        "p95_ns": nearest_rank(measured, 0.95),
        "min_ns": min(measured),
        "max_ns": max(measured),
        "warnings": warnings,
        "host_swap_delta_pages": environment["end"]["swap_pages"]
        - environment["start"]["swap_pages"],
        "raw_sha256": raw_hashes,
        "first_raw": first_raw,
        "peak_rss_kib": [item["peak_rss_kib"] for item in resources[1:]],
        "user_cpu_s": [item["user_cpu_s"] for item in resources[1:]],
        "system_cpu_s": [item["system_cpu_s"] for item in resources[1:]],
        "fs_inputs": [item["fs_inputs"] for item in resources[1:]],
        "fs_outputs": [item["fs_outputs"] for item in resources[1:]],
        "major_faults": [item["major_faults"] for item in resources[1:]],
        "swap_events": [item["swap_events"] for item in resources[1:]],
    }


def recompute(
    root: Path,
    protocol_path: Path,
    baseline_wheel: Path,
    candidate_wheel: Path,
    pairs: int,
) -> dict:
    """Recompute complete pairs; report an interim result until all five exist."""
    protocol = json.loads(protocol_path.read_text())
    if protocol["status"] != "FROZEN_S02_PYTHON_PAIRED" or not 1 <= pairs <= 5:
        raise ValueError("protocol or pair count invalid")
    protocol_sha256 = sha(protocol_path)
    run = json.loads((root / "run-manifest.json").read_text())
    if (
        run["protocol_sha256"] != protocol_sha256
        or run["pair_order"] != protocol["pair_order"]
    ):
        raise ValueError("paired run manifest differs from protocol")
    wheels = {"baseline": baseline_wheel, "candidate": candidate_wheel}
    pair_results = []
    all_samples = {"baseline": [], "candidate": []}
    resource_rows = {
        role: {key: [] for key in RESOURCE_KEYS.values()}
        for role in ("baseline", "candidate")
    }
    raw_hashes = {}
    first_raw = None
    for pair in range(1, pairs + 1):
        blocks = {}
        for role in protocol["pair_order"][pair - 1]:
            block = audit_block(
                root / f"pair-{pair:02d}-{role}",
                protocol=protocol,
                protocol_sha256=protocol_sha256,
                role=role,
                wheel=wheels[role],
            )
            blocks[role] = block
            all_samples[role].extend(block["samples_ns"])
            raw_hashes.update(block["raw_sha256"])
            for key in resource_rows[role]:
                resource_rows[role][key].extend(block[key])
            if first_raw is None:
                first_raw = block["first_raw"]
        pair_results.append(
            {
                "pair": pair,
                "order": protocol["pair_order"][pair - 1],
                "baseline_p50_ns": blocks["baseline"]["p50_ns"],
                "candidate_p50_ns": blocks["candidate"]["p50_ns"],
                "baseline_p95_ns": blocks["baseline"]["p95_ns"],
                "candidate_p95_ns": blocks["candidate"]["p95_ns"],
                "p50_delta_percent": 100
                * (blocks["candidate"]["p50_ns"] - blocks["baseline"]["p50_ns"])
                / blocks["baseline"]["p50_ns"],
                "p95_delta_percent": 100
                * (blocks["candidate"]["p95_ns"] - blocks["baseline"]["p95_ns"])
                / blocks["baseline"]["p95_ns"],
                "baseline_warnings": blocks["baseline"]["warnings"],
                "candidate_warnings": blocks["candidate"]["warnings"],
            }
        )
    assert first_raw is not None
    negative = deepcopy(first_raw)
    negative["observed"]["after_reopen"]["source_absent"] = False
    try:
        validate_materialized(negative)
    except ValueError as error:
        negative_message = str(error)
    else:
        raise AssertionError("surviving source was accepted")
    if negative_message != "after_reopen source_absent failed":
        raise ValueError("negative control rejected for wrong reason")
    result = {
        "schema_version": 1,
        "status": "INTERIM_PAIRED" if pairs < 5 else "FULL_PYTHON_S02_PAIRED",
        "protocol_sha256": protocol_sha256,
        "pairs": pair_results,
        "sample_count_per_version": {
            role: len(values) for role, values in all_samples.items()
        },
        "raw_sha256": raw_hashes,
        "negative_control_rejected": negative_message,
        "resources": {
            role: {
                "peak_rss_kib_min": min(resource_rows[role]["peak_rss_kib"]),
                "peak_rss_kib_max": max(resource_rows[role]["peak_rss_kib"]),
                "user_cpu_s_median": statistics.median(
                    resource_rows[role]["user_cpu_s"]
                ),
                "system_cpu_s_median": statistics.median(
                    resource_rows[role]["system_cpu_s"]
                ),
                "fs_inputs_total": sum(resource_rows[role]["fs_inputs"]),
                "fs_outputs_total": sum(resource_rows[role]["fs_outputs"]),
                "major_faults_total": sum(resource_rows[role]["major_faults"]),
                "swap_events_total": sum(resource_rows[role]["swap_events"]),
            }
            for role in ("baseline", "candidate")
        },
    }
    if pairs == 5:
        result["whole_product"] = {
            role: {
                "p50_ns": nearest_rank(values, 0.5),
                "p95_ns": nearest_rank(values, 0.95),
                "min_ns": min(values),
                "max_ns": max(values),
            }
            for role, values in all_samples.items()
        }
        result["pair_delta_spread_percent"] = {
            name: {
                "median": statistics.median(pair[name] for pair in pair_results),
                "min": min(pair[name] for pair in pair_results),
                "max": max(pair[name] for pair in pair_results),
            }
            for name in ("p50_delta_percent", "p95_delta_percent")
        }
        warning_free = [
            pair
            for pair in pair_results
            if not pair["baseline_warnings"] and not pair["candidate_warnings"]
        ]
        result["warning_free_pairs"] = len(warning_free)
        result["warning_free_pair_delta_percent"] = {
            name: [pair[name] for pair in warning_free]
            for name in ("p50_delta_percent", "p95_delta_percent")
        }
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--baseline-wheel", type=Path, required=True)
    parser.add_argument("--candidate-wheel", type=Path, required=True)
    parser.add_argument("--pairs", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = recompute(
        args.root, args.protocol, args.baseline_wheel, args.candidate_wheel, args.pairs
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"status": result["status"], "pairs": len(result["pairs"])}))


if __name__ == "__main__":
    main()
