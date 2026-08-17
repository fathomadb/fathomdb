#!/usr/bin/env python3
"""Fail-closed verifier for an exact CUDA package-rehearsal evidence bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, NoReturn


SCHEMA_VERSION = "fathomdb.cuda-package-rehearsal/v1"
BUILD_INPUT_SCHEMA = "fathomdb.cuda-package-build-input/v1"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
COMMIT_SHA = re.compile(r"[0-9a-f]{40}\Z")
MANIFEST = "cuda-package-rehearsal.json"
ROUTE_RECEIPT = "route-receipt.json"
BUILD_INPUT = "build-input.json"
PACKAGE_DIR = "packages"
SMOKE_DIR = "smoke"
SMOKE_NAMES = frozenset({"cpu-python.json", "cpu-napi.json", "gpu-python.json", "gpu-napi.json"})


def fail(message: str) -> NoReturn:
    print(f"cuda-package-rehearsal: {message}", file=sys.stderr)
    raise SystemExit(1)


def canonical_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require_sha(value: object, label: str, pattern: re.Pattern[str] = SHA256) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        fail(f"{label} must be lowercase hexadecimal")
    return value


def load_object(path: Path, label: str) -> tuple[dict[str, Any], bytes]:
    if path.is_symlink():
        fail(f"{label} must not be a symlink")
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {label}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    if raw != canonical_json(value):
        fail(f"{label} is not canonical JSON")
    return value, raw


def require_exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    if set(value) != expected:
        fail(f"{label} has missing or unknown fields")


def require_regular_directory(path: Path, label: str) -> None:
    if path.is_symlink() or not path.is_dir():
        fail(f"{label} must be a non-symlink directory")


def package_kind(name: str) -> str:
    if "/" in name or name in {"", ".", ".."}:
        fail("package filename is not a plain filename")
    if name.startswith("fathomdb-linux-x64-gnu-") and name.endswith(".tgz"):
        return "napi"
    if name.startswith("fathomdb-") and name.endswith(".whl"):
        return "python"
    if name.startswith("fathomdb-") and name.endswith(".tgz"):
        return "npm"
    fail(f"package filename is not an allowed retained artifact: {name}")


def validate_build_input(value: object, candidate_sha: str) -> None:
    if not isinstance(value, dict):
        fail("build input is not an object")
    require_exact_keys(
        value,
        {"schema_version", "candidate_sha", "python_features", "napi_features", "rerank_cuda"},
        "build input",
    )
    if value["schema_version"] != BUILD_INPUT_SCHEMA:
        fail("build input schema is unsupported")
    if require_sha(value["candidate_sha"], "build input candidate SHA", COMMIT_SHA) != candidate_sha:
        fail("build input candidate SHA does not bind the rehearsal candidate")
    if value["python_features"] != ["embed-cuda", "pyo3/extension-module"]:
        fail("Python package was not built with the exact embed-cuda feature set")
    if value["napi_features"] != ["default-embedder", "embed-cuda"]:
        fail("N-API package was not built with the exact embed-cuda feature set")
    if value["rerank_cuda"] is not False:
        fail("CUDA reranker must not be included in package rehearsal")


def validate_cpu_smoke(value: dict[str, Any], consumer: str) -> None:
    require_exact_keys(
        value,
        {"schema_version", "consumer", "network", "environment", "gpu_nodes_visible", "source_imported", "outcome"},
        f"CPU {consumer} smoke",
    )
    if value != {
        "schema_version": "fathomdb.cuda-package-cpu-smoke/v1",
        "consumer": consumer,
        "network": "none",
        "environment": "env -i",
        "gpu_nodes_visible": False,
        "source_imported": False,
        "outcome": "passed",
    }:
        fail(f"CPU {consumer} smoke does not prove the driverless installed-artifact contract")


def validate_gpu_smoke(value: dict[str, Any], consumer: str) -> None:
    require_exact_keys(
        value,
        {
            "schema_version", "consumer", "network", "source_imported", "outcome", "gpu_uuid", "host_index",
            "device_name", "driver_version", "requested_ordinal", "smoke_pid", "nvidia_smi_pid", "nvidia_smi_uuid",
        },
        f"GPU {consumer} smoke",
    )
    if value["schema_version"] != "fathomdb.cuda-package-gpu-smoke/v1" or value["consumer"] != consumer:
        fail(f"GPU {consumer} smoke schema or consumer differs")
    if value["network"] != "none" or value["source_imported"] is not False or value["outcome"] != "passed":
        fail(f"GPU {consumer} smoke is not an isolated installed-artifact success")
    for name in ("gpu_uuid", "device_name", "driver_version"):
        if not isinstance(value[name], str) or not value[name]:
            fail(f"GPU {consumer} smoke lacks {name}")
    for name in ("host_index", "requested_ordinal"):
        if not isinstance(value[name], int) or isinstance(value[name], bool) or value[name] < 0:
            fail(f"GPU {consumer} smoke has invalid {name}")
    for name in ("smoke_pid", "nvidia_smi_pid"):
        if not isinstance(value[name], int) or isinstance(value[name], bool) or value[name] < 1:
            fail(f"GPU {consumer} smoke has invalid {name}")
    if value["gpu_uuid"] != value["nvidia_smi_uuid"] or value["smoke_pid"] != value["nvidia_smi_pid"]:
        fail(f"GPU {consumer} smoke lacks GPU UUID/PID correlation")


def validate_smokes(root: Path, expected: object) -> None:
    if not isinstance(expected, dict) or set(expected) != SMOKE_NAMES:
        fail("smoke evidence inventory is incomplete or contains unknown evidence")
    smoke_dir = root / SMOKE_DIR
    require_regular_directory(smoke_dir, "smoke evidence directory")
    actual_names = {path.name for path in smoke_dir.iterdir()}
    if actual_names != SMOKE_NAMES:
        fail("smoke evidence directory does not exactly match manifest inventory")
    for name in sorted(SMOKE_NAMES):
        path = smoke_dir / name
        if path.is_symlink() or not path.is_file():
            fail(f"smoke evidence must be a regular file: {name}")
        if require_sha(expected[name], f"smoke evidence digest {name}") != sha256(path):
            fail(f"smoke evidence digest mismatch: {name}")
        value, _ = load_object(path, f"smoke evidence {name}")
        kind, consumer = name.removesuffix(".json").split("-", 1)
        if kind == "cpu":
            validate_cpu_smoke(value, consumer)
        else:
            validate_gpu_smoke(value, consumer)


def validate(root: Path, candidate_sha: str) -> None:
    if require_sha(candidate_sha, "requested candidate SHA", COMMIT_SHA) != candidate_sha:
        fail("unreachable")
    require_regular_directory(root, "rehearsal directory")
    allowed_root = {MANIFEST, ROUTE_RECEIPT, BUILD_INPUT, PACKAGE_DIR, SMOKE_DIR}
    if {path.name for path in root.iterdir()} != allowed_root:
        fail("rehearsal directory has missing or unknown members")
    manifest, _ = load_object(root / MANIFEST, "rehearsal manifest")
    require_exact_keys(
        manifest,
        {"schema_version", "candidate_sha", "route_receipt_sha256", "preflight_witness_sha256", "build_input", "packages", "smoke_evidence_sha256"},
        "rehearsal manifest",
    )
    if manifest["schema_version"] != SCHEMA_VERSION:
        fail("rehearsal manifest schema is unsupported")
    if require_sha(manifest["candidate_sha"], "rehearsal candidate SHA", COMMIT_SHA) != candidate_sha:
        fail("rehearsal manifest candidate SHA does not match the requested candidate")
    route, route_bytes = load_object(root / ROUTE_RECEIPT, "route receipt")
    if route.get("schema_version") != "fathomdb.cuda-unmerged-route-receipt/v1" or route.get("candidate_sha") != candidate_sha:
        fail("route receipt does not bind the requested candidate")
    if require_sha(manifest["route_receipt_sha256"], "route receipt digest") != hashlib.sha256(route_bytes).hexdigest():
        fail("route receipt digest mismatch")
    require_sha(manifest["preflight_witness_sha256"], "preflight witness digest")

    build_input_path = root / BUILD_INPUT
    build_input, build_input_bytes = load_object(build_input_path, "build input")
    if manifest["build_input"] != build_input:
        fail("manifest build input differs from the retained build input")
    validate_build_input(build_input, candidate_sha)
    _ = build_input_bytes

    packages = manifest["packages"]
    if not isinstance(packages, dict) or len(packages) != 3:
        fail("package inventory must contain exactly three retained artifacts")
    package_dir = root / PACKAGE_DIR
    require_regular_directory(package_dir, "package directory")
    if {path.name for path in package_dir.iterdir()} != set(packages):
        fail("package directory does not exactly match manifest inventory")
    kinds: set[str] = set()
    for name, expected_digest in packages.items():
        if not isinstance(name, str):
            fail("package filename is invalid")
        kinds.add(package_kind(name))
        path = package_dir / name
        if path.is_symlink() or not path.is_file():
            fail(f"package must be a regular non-symlink file: {name}")
        if require_sha(expected_digest, f"package digest {name}") != sha256(path):
            fail(f"package digest mismatch: {name}")
    if kinds != {"python", "npm", "napi"}:
        fail("package inventory must contain exactly Python, thin npm, and Linux-x64 N-API artifacts")
    validate_smokes(root, manifest["smoke_evidence_sha256"])


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rehearsal-dir", required=True, type=Path)
    parser.add_argument("--candidate-sha", required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    validate(args.rehearsal_dir, args.candidate_sha)
    print("cuda-package-rehearsal: pass")


if __name__ == "__main__":
    main()
