#!/usr/bin/env python3
"""Validate and aggregate the installed Slice 72 CE profile."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import statistics
import sys
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
from slice72_ce_artifact import ArtifactError, validate_receipt


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")


class ProfileError(ValueError):
    """A profile input cannot support a passing receipt."""


def exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    unknown = set(value) - expected
    missing = expected - set(value)
    if unknown:
        raise ProfileError(f"{label} has unknown keys: {sorted(unknown)}")
    if missing:
        raise ProfileError(f"{label} is missing keys: {sorted(missing)}")


def canonical_digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


def receipt_digest(value: Any) -> str:
    payload = (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()
    return hashlib.sha256(payload).hexdigest()


def nearest_rank(values: list[int], quantile: float) -> int:
    """Return the preregistered nearest-rank percentile."""
    if not values or not 0 < quantile <= 1:
        raise ProfileError("nearest-rank requires values and 0 < quantile <= 1")
    ordered = sorted(values)
    return ordered[math.ceil(quantile * len(ordered)) - 1]


def validate_manifest(manifest: dict[str, Any]) -> None:
    exact_keys(manifest, {
        "schema_version", "baseline_sha", "candidate_sha", "fixture", "features",
        "model_sha256", "cold_repetitions", "steady_repetitions", "steady_calls",
        "score_tolerance", "p95_regression_ratio", "environment", "cpu_affinity", "timer", "cuda",
    }, "manifest")
    if manifest["schema_version"] != "fathomdb.slice72.ce-profile-manifest/v1":
        raise ProfileError("unsupported manifest schema")
    for key in ["baseline_sha", "candidate_sha"]:
        if not isinstance(manifest[key], str) or not COMMIT_RE.fullmatch(manifest[key]):
            raise ProfileError(f"manifest {key} must be a full commit SHA")
    if manifest["baseline_sha"] == manifest["candidate_sha"]:
        raise ProfileError("baseline and candidate commits must differ")
    exact_keys(manifest["fixture"], {
        "query", "passages", "passage_ids", "standalone_order", "engine_ids",
        "engine_ties", "rerank_depth", "pool_n", "alpha",
    }, "fixture")
    fixture = manifest["fixture"]
    if sorted(fixture["passage_ids"]) != sorted(fixture["standalone_order"]):
        raise ProfileError("standalone order must contain every fixture passage ID")
    if len(set(fixture["passage_ids"])) != len(fixture["passage_ids"]):
        raise ProfileError("fixture passage IDs must be unique")
    if not isinstance(fixture["query"], str) or not fixture["query"]:
        raise ProfileError("fixture query must be non-empty")
    if not isinstance(fixture["passages"], list) or [item.get("id") for item in fixture["passages"]] != fixture["passage_ids"]:
        raise ProfileError("fixture passages must match passage IDs in order")
    if fixture["rerank_depth"] != len(fixture["passages"]) or fixture["pool_n"] != len(fixture["passages"]) or fixture["alpha"] != 1.0:
        raise ProfileError("fixture rerank parameters must cover the pool at alpha 1.0")
    expected_features = {
        "cpu": ["pyo3/extension-module", "default-reranker"],
        "cuda": ["pyo3/extension-module", "rerank-cuda"],
    }
    if manifest["features"] != expected_features:
        raise ProfileError("manifest feature sets do not match Slice 72")
    if not isinstance(manifest["cuda"], dict):
        raise ProfileError("manifest CUDA selection is missing")
    exact_keys(manifest["cuda"], {"selected_uuid", "visible_selector", "model"}, "manifest cuda")
    if (
        not isinstance(manifest["cuda"]["selected_uuid"], str)
        or manifest["cuda"]["visible_selector"] != manifest["cuda"]["selected_uuid"]
        or manifest["cuda"]["model"] != "NVIDIA GeForce RTX 3090"
    ):
        raise ProfileError("manifest must pin one RTX 3090 by UUID")
    if set(manifest["model_sha256"]) != {"config.json", "tokenizer.json", "model.safetensors"} or any(
        not isinstance(value, str) or not SHA256_RE.fullmatch(value)
        for value in manifest["model_sha256"].values()
    ):
        raise ProfileError("manifest model hashes are incomplete or malformed")
    for key in ["cold_repetitions", "steady_repetitions", "steady_calls"]:
        if not isinstance(manifest[key], int) or manifest[key] <= 0:
            raise ProfileError(f"manifest {key} must be positive")
    if not isinstance(manifest["score_tolerance"], (int, float)) or not 0 <= manifest["score_tolerance"] <= 0.1:
        raise ProfileError("manifest score tolerance is invalid")
    if not isinstance(manifest["p95_regression_ratio"], (int, float)) or not 1 <= manifest["p95_regression_ratio"] <= 2:
        raise ProfileError("manifest p95 regression ratio is invalid")
    if manifest["environment"] != {
        "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1",
        "MKL_NUM_THREADS": "1", "RAYON_NUM_THREADS": "1",
    }:
        raise ProfileError("manifest thread environment is not pinned")
    if not isinstance(manifest["cpu_affinity"], list) or not manifest["cpu_affinity"]:
        raise ProfileError("manifest CPU affinity is not pinned")
    if manifest["timer"] != "time.perf_counter_ns/nearest-rank":
        raise ProfileError("manifest timer contract is invalid")


def validate_output(output: Any, expected_ids: list[int], label: str) -> tuple[list[int], dict[str, float]]:
    if not isinstance(output, dict):
        raise ProfileError(f"{label} output must be an object")
    exact_keys(output, {"ids", "ce_scores"}, f"{label} output")
    if output["ids"] != expected_ids:
        kind = "standalone order" if "standalone" in label else "Engine IDs/order"
        raise ProfileError(f"{kind} does not match the manifest")
    scores = output["ce_scores"]
    if not isinstance(scores, dict) or set(scores) != {str(item) for item in expected_ids}:
        raise ProfileError(f"{label} CE scores do not cover the expected IDs")
    numeric: dict[str, float] = {}
    for key, value in scores.items():
        if not isinstance(value, (int, float)) or not math.isfinite(value):
            raise ProfileError(f"{label} CE scores must be finite")
        numeric[key] = float(value)
    if len(set(numeric.values())) < 2:
        raise ProfileError(f"{label} CE scores must be non-degenerate")
    return list(output["ids"]), numeric


def validate_runtime(manifest: dict[str, Any], cell: dict[str, Any], runtime: Any, label: str) -> None:
    if not isinstance(runtime, dict):
        raise ProfileError(f"{label} runtime must be an object")
    exact_keys(runtime, {
        "pid", "install_root", "module_path", "native_path", "native_sha256",
        "source_imported", "effective_device", "selected_uuid", "allocation",
        "import_ns", "open_ns", "peak_rss_kib", "affinity",
    }, f"{label} runtime")
    artifact = cell["artifact"]
    if runtime["install_root"] != artifact["venv_root"] or runtime["module_path"] != artifact["module_path"] or runtime["native_path"] != artifact["native_path"]:
        raise ProfileError(f"{label} runtime did not use the installed artifact")
    if runtime["native_sha256"] != artifact["native_sha256"] or runtime["source_imported"] is not False:
        raise ProfileError(f"{label} runtime native/source identity is invalid")
    if runtime["affinity"] != manifest["cpu_affinity"]:
        raise ProfileError(f"{label} runtime CPU affinity changed")
    for key in ["pid", "import_ns", "open_ns", "peak_rss_kib"]:
        if not isinstance(runtime[key], int) or runtime[key] <= 0:
            raise ProfileError(f"{label} runtime {key} must be positive")
    if cell["device"] == "cpu":
        if runtime["effective_device"] != "cpu" or runtime["selected_uuid"] is not None or runtime["allocation"] is not None:
            raise ProfileError(f"{label} CPU runtime device evidence is invalid")
    else:
        allocation = runtime["allocation"]
        if runtime["effective_device"] != "cuda:0" or runtime["selected_uuid"] != manifest["cuda"]["selected_uuid"] or not isinstance(allocation, dict):
            raise ProfileError(f"{label} CUDA runtime device/allocation evidence is missing")
        exact_keys(allocation, {"pid", "gpu_uuid", "vram_mib"}, f"{label} allocation")
        if allocation["pid"] != runtime["pid"] or allocation["gpu_uuid"] != runtime["selected_uuid"] or not isinstance(allocation["vram_mib"], (int, float)) or allocation["vram_mib"] <= 0:
            raise ProfileError(f"{label} CUDA allocation does not bind its PID and UUID")


def validate_path(manifest: dict[str, Any], cell: dict[str, Any], path_name: str) -> dict[str, Any]:
    path = cell["paths"][path_name]
    exact_keys(path, {"cold", "steady"}, f"{path_name} path")
    if len(path["cold"]) != manifest["cold_repetitions"]:
        raise ProfileError(f"{path_name} cold repetitions do not match the manifest")
    if len(path["steady"]) != manifest["steady_repetitions"]:
        raise ProfileError(f"{path_name} steady repetitions do not match the manifest")
    expected_ids = manifest["fixture"]["standalone_order" if path_name == "standalone" else "engine_ids"]
    reference: tuple[list[int], dict[str, float]] | None = None
    cold_ns: list[int] = []
    repetition_p95_ns: list[int] = []
    all_steady_ns: list[int] = []
    for mode, records in [("cold", path["cold"]), ("steady", path["steady"])]:
        for index, repetition in enumerate(records):
            label = f"{path_name} {mode} {index}"
            expected_keys = {"duration_ns", "output", "output_digests", "runtime", "raw_sha256"} if mode == "cold" else {"durations_ns", "output", "output_digests", "runtime", "raw_sha256"}
            if not isinstance(repetition, dict):
                raise ProfileError(f"{label} repetition must be an object")
            exact_keys(repetition, expected_keys, f"{label} repetition")
            validate_runtime(manifest, cell, repetition["runtime"], label)
            if not isinstance(repetition["raw_sha256"], str) or not SHA256_RE.fullmatch(repetition["raw_sha256"]):
                raise ProfileError(f"{label} raw evidence digest is malformed")
            current = validate_output(repetition["output"], expected_ids, label)
            reference = reference or current
            if current != reference:
                raise ProfileError(f"{path_name} output is not stable across repetitions")
            digests = repetition["output_digests"]
            expected_count = 1 if mode == "cold" else manifest["steady_calls"]
            expected_digest = canonical_digest(repetition["output"])
            if not isinstance(digests, list) or len(digests) != expected_count or any(value != expected_digest for value in digests):
                raise ProfileError(f"{label} per-call outputs are not stable")
            if mode == "cold":
                duration = repetition["duration_ns"]
                if not isinstance(duration, int) or duration <= 0:
                    raise ProfileError(f"{label} duration must be positive")
                cold_ns.append(duration)
            else:
                durations = repetition["durations_ns"]
                if not isinstance(durations, list) or len(durations) != manifest["steady_calls"] or any(not isinstance(value, int) or value <= 0 for value in durations):
                    raise ProfileError(f"{label} steady durations are invalid")
                repetition_p95_ns.append(nearest_rank(durations, 0.95))
                all_steady_ns.extend(durations)
    assert reference is not None
    return {
        "cold_ns": cold_ns,
        "steady_p50_ns": nearest_rank(all_steady_ns, 0.50),
        "steady_p95_ns": nearest_rank(all_steady_ns, 0.95),
        "steady_p99_ns": nearest_rank(all_steady_ns, 0.99),
        "median_repetition_p95_ns": statistics.median(repetition_p95_ns),
        "throughput_per_second": len(all_steady_ns) * 1_000_000_000 / sum(all_steady_ns),
        "output": {"ids": reference[0], "ce_scores": reference[1]},
    }


def validate_cell(manifest: dict[str, Any], cell: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(cell, dict):
        raise ProfileError("profile cell must be an object")
    exact_keys(cell, {
        "schema_version", "role", "device", "commit_sha", "features",
        "artifact_receipt_sha256", "artifact", "network_policy", "cuda_inventory",
        "model_sha256_before", "model_sha256_after", "model_files_read_only", "paths",
    }, "profile cell")
    if cell["schema_version"] != "fathomdb.slice72.ce-profile-cell/v2":
        raise ProfileError("unsupported profile cell schema")
    role, device = cell["role"], cell["device"]
    if role not in {"baseline", "candidate"} or device not in {"cpu", "cuda"}:
        raise ProfileError("cell role/device is invalid")
    if cell["commit_sha"] != manifest[f"{role}_sha"] or cell["features"] != manifest["features"][device]:
        raise ProfileError("cell commit/features do not match the manifest")
    try:
        validate_receipt(cell["artifact"], manifest, role, device, verify_files=False)
    except ArtifactError as exc:
        raise ProfileError(str(exc)) from exc
    if cell["artifact_receipt_sha256"] != receipt_digest(cell["artifact"]):
        raise ProfileError("artifact receipt digest does not match the embedded receipt")
    if cell["network_policy"] != "offline-loopback-proxy" or cell["model_files_read_only"] is not True:
        raise ProfileError("cell did not enforce the offline read-only model policy")
    if device == "cpu":
        if cell["cuda_inventory"] is not None:
            raise ProfileError("CPU cell must not contain CUDA inventory")
    elif cell["cuda_inventory"] != {
        "uuid": manifest["cuda"]["selected_uuid"],
        "model": manifest["cuda"]["model"],
    }:
        raise ProfileError("CUDA cell does not bind the selected RTX 3090")
    if cell["model_sha256_before"] != manifest["model_sha256"] or cell["model_sha256_after"] != manifest["model_sha256"]:
        raise ProfileError("cell model identity changed or differs from the manifest")
    if not isinstance(cell["paths"], dict) or set(cell["paths"]) != {"standalone", "engine"}:
        raise ProfileError("cell paths must contain standalone and engine")
    return {path: validate_path(manifest, cell, path) for path in ["standalone", "engine"]}


def validate_and_aggregate(manifest: dict[str, Any], cells: list[dict[str, Any]]) -> dict[str, Any]:
    """Validate all cells and return a recomputed PASS receipt."""
    validate_manifest(manifest)
    expected = {(role, device) for role in ["baseline", "candidate"] for device in ["cpu", "cuda"]}
    actual = {(cell.get("role"), cell.get("device")) for cell in cells if isinstance(cell, dict)}
    if len(cells) != 4 or actual != expected:
        raise ProfileError("profile does not contain the complete cell set")
    summaries = {(cell["role"], cell["device"]): validate_cell(manifest, cell) for cell in cells}
    tolerance = float(manifest["score_tolerance"])
    for role in ["baseline", "candidate"]:
        for path in ["standalone", "engine"]:
            cpu = summaries[(role, "cpu")][path]["output"]
            cuda = summaries[(role, "cuda")][path]["output"]
            if cpu["ids"] != cuda["ids"]:
                raise ProfileError(f"CPU/CUDA {path} rank order differs")
            for item in cpu["ids"]:
                key = str(item)
                if abs(cpu["ce_scores"][key] - cuda["ce_scores"][key]) > tolerance:
                    raise ProfileError(f"CPU/CUDA {path} CE scores exceed tolerance")
    comparisons: list[dict[str, Any]] = []
    limit = float(manifest["p95_regression_ratio"])
    for device in ["cpu", "cuda"]:
        for path in ["standalone", "engine"]:
            baseline = summaries[("baseline", device)][path]["median_repetition_p95_ns"]
            candidate = summaries[("candidate", device)][path]["median_repetition_p95_ns"]
            ratio = candidate / baseline
            if ratio > limit:
                raise ProfileError(f"{device} {path} p95 regression {ratio:.6f} exceeds {limit:.6f}")
            comparisons.append({
                "device": device, "path": path,
                "baseline_median_p95_ns": baseline,
                "candidate_median_p95_ns": candidate, "ratio": ratio,
            })
    evidence = []
    for cell in cells:
        artifact = cell["artifact"]
        evidence.append({
            "role": cell["role"],
            "device": cell["device"],
            "cell_canonical_sha256": canonical_digest(cell),
            "artifact_receipt_sha256": cell["artifact_receipt_sha256"],
            "wheel_sha256": artifact["wheel_sha256"],
            "native_sha256": artifact["native_sha256"],
            "module_path": artifact["module_path"],
            "native_path": artifact["native_path"],
            "model_sha256": cell["model_sha256_after"],
            "cuda_inventory": cell["cuda_inventory"],
            "raw_sha256": [
                repetition["raw_sha256"]
                for path in ["standalone", "engine"]
                for mode in ["cold", "steady"]
                for repetition in cell["paths"][path][mode]
            ],
        })
    return {
        "schema_version": "fathomdb.slice72.ce-profile-receipt/v1",
        "verdict": "PASS", "baseline_sha": manifest["baseline_sha"],
        "candidate_sha": manifest["candidate_sha"],
        "manifest_canonical_sha256": canonical_digest(manifest),
        "environment": manifest["environment"],
        "evidence": evidence,
        "summaries": {f"{role}-{device}": summary for (role, device), summary in summaries.items()},
        "comparisons": comparisons,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--cells", type=Path, nargs="+", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
        cells = [json.loads(path.read_text(encoding="utf-8")) for path in args.cells]
        receipt = validate_and_aggregate(manifest, cells)
    except (OSError, json.JSONDecodeError, ProfileError) as exc:
        print(f"FAIL Slice 72 CE profile: {exc}", file=sys.stderr)
        return 1
    payload = json.dumps(receipt, sort_keys=True, separators=(",", ":")) + "\n"
    if args.output:
        args.output.write_text(payload, encoding="utf-8")
    else:
        sys.stdout.write(payload)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
