#!/usr/bin/env python3
"""Fail-closed verifier for an exact CUDA package-rehearsal evidence bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tarfile
from pathlib import Path
from typing import Any, NoReturn


SCHEMA_VERSION_V2 = "fathomdb.cuda-package-rehearsal/v2"
SCHEMA_VERSION_V3 = "fathomdb.cuda-package-rehearsal/v3"
BUILD_INPUT_SCHEMA_V2 = "fathomdb.cuda-package-build-input/v2"
BUILD_INPUT_SCHEMA_V3 = "fathomdb.cuda-package-build-input/v3"
UNMERGED_ROUTE_SCHEMA = "fathomdb.cuda-unmerged-route-receipt/v1"
MAIN_ROUTE_SCHEMA = "fathomdb.cuda-main-route-receipt/v2"
MAIN_WORKFLOW_REF = "fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
COMMIT_SHA = re.compile(r"[0-9a-f]{40}\Z")
MANIFEST = "cuda-package-rehearsal.json"
ROUTE_RECEIPT = "route-receipt.json"
BUILD_INPUT = "build-input.json"
PREFLIGHT_WITNESS_DIR = "preflight-witness"
PREFLIGHT_WITNESS = "cuda-preflight-witness.json"
PACKAGE_DIR = "packages"
SMOKE_DIR = "smoke"
BASE_SMOKE_NAMES = frozenset({
    "cpu-python.json", "cpu-napi.json", "gpu-python.json", "gpu-napi.json",
    "cpu-cli.json", "cpu-cli-stdout.json",
    "forced-cuda-unavailable-cli.json", "forced-cuda-unavailable-cli-stdout.json",
})
RERANK_V3_SMOKE_NAMES = frozenset({"reranker-cli-doctor.json", "reranker-cli-doctor-stdout.json"})
PACKAGE_KINDS = frozenset({"python_wheel", "npm_main", "napi_platform", "cli_archive"})
TARGET = "x86_64-unknown-linux-gnu"


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


def load_preflight_witness(path: Path) -> tuple[dict[str, Any], bytes]:
    if path.is_symlink():
        fail("preflight witness must not be a symlink")
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read preflight witness: {error}")
    if not isinstance(value, dict):
        fail("preflight witness must be a JSON object")
    canonical = canonical_json(value)
    if raw != canonical:
        fail("preflight witness is not canonical JSON")
    return value, raw


def require_exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    if set(value) != expected:
        fail(f"{label} has missing or unknown fields")


def require_regular_directory(path: Path, label: str) -> None:
    if path.is_symlink() or not path.is_dir():
        fail(f"{label} must be a non-symlink directory")


def validate_build_input(value: object, candidate_sha: str, version: str) -> str:
    if not isinstance(value, dict):
        fail("build input is not an object")
    require_exact_keys(
        value,
        {"schema_version", "candidate_sha", "version", "target", "python_features", "napi_features", "cli_features", "rerank_cuda", "model_cache_manifest_sha256", "archive_filename"},
        "build input",
    )
    schema = value["schema_version"]
    if schema not in {BUILD_INPUT_SCHEMA_V2, BUILD_INPUT_SCHEMA_V3}:
        fail("build input schema is unsupported")
    if require_sha(value["candidate_sha"], "build input candidate SHA", COMMIT_SHA) != candidate_sha:
        fail("build input candidate SHA does not bind the rehearsal candidate")
    if value["version"] != version or value["target"] != TARGET:
        fail("build input version or target differs from the candidate package identity")
    rerank = schema == BUILD_INPUT_SCHEMA_V3
    if value["python_features"] != (["embed-cuda", "rerank-cuda", "pyo3/extension-module"] if rerank else ["embed-cuda", "pyo3/extension-module"]):
        fail("Python package was not built with the exact embed-cuda feature set")
    if value["napi_features"] != (["default-embedder", "embed-cuda", "rerank-cuda"] if rerank else ["default-embedder", "embed-cuda"]):
        fail("N-API package was not built with the exact embed-cuda feature set")
    if value["cli_features"] != (["embed-cuda", "rerank-cuda"] if rerank else ["embed-cuda"]):
        fail("CLI package was not built with the exact embed-cuda feature set")
    if value["rerank_cuda"] is not rerank:
        fail("build input rerank CUDA flag does not match its schema")
    if value["archive_filename"] != f"fathomdb-{version}-{TARGET}.tar.gz":
        fail("build input CLI archive coordinate differs from the candidate version")
    require_sha(value["model_cache_manifest_sha256"], "model cache manifest digest")
    return schema


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


def validate_cli_archive(path: Path, version: str) -> None:
    expected_root = f"fathomdb-{version}-{TARGET}"
    try:
        raw = path.read_bytes()
        if len(raw) < 10 or raw[:2] != b"\x1f\x8b" or raw[4:8] != b"\0\0\0\0":
            fail("CLI archive is not deterministic gzip -n output")
        with tarfile.open(path, mode="r:gz", format=tarfile.PAX_FORMAT) as archive:
            members = archive.getmembers()
    except (OSError, tarfile.TarError) as error:
        fail(f"cannot inspect CLI archive: {error}")
    if [member.name.rstrip("/") for member in members] != [expected_root, f"{expected_root}/fathomdb"]:
        fail("CLI archive must contain exactly its versioned root and fathomdb leaf")
    directory, binary = members
    if not directory.isdir() or not binary.isfile() or binary.islnk() or binary.issym():
        fail("CLI archive contains a non-regular entry")
    for member in members:
        if member.uid != 0 or member.gid != 0 or member.uname or member.gname or member.mtime != 0:
            fail("CLI archive owner or timestamp metadata is not deterministic")
        if member.pax_headers.get("atime") or member.pax_headers.get("ctime"):
            fail("CLI archive retains variable pax timestamps")
    if directory.mode != 0o755 or binary.mode != 0o755:
        fail("CLI archive entries must have mode 0755")


def validate_package_coordinate(kind: str, value: object, version: str, package_dir: Path) -> str:
    if not isinstance(value, dict):
        fail(f"package coordinate {kind} must be an object")
    require_exact_keys(value, {"contains_cuda", "filename", "sha256", "target", "version"}, f"package coordinate {kind}")
    expected_names = {
        "python_wheel": re.compile(rf"fathomdb-{re.escape(version)}-.+\.whl\Z"),
        "npm_main": re.compile(rf"fathomdb-{re.escape(version)}\.tgz\Z"),
        "napi_platform": re.compile(rf"fathomdb-linux-x64-gnu-{re.escape(version)}\.tgz\Z"),
        "cli_archive": re.compile(rf"fathomdb-{re.escape(version)}-{TARGET}\.tar\.gz\Z"),
    }
    filename = value["filename"]
    if not isinstance(filename, str) or "/" in filename or expected_names[kind].fullmatch(filename) is None:
        fail(f"package coordinate {kind} has invalid filename")
    if value["version"] != version or value["target"] != TARGET:
        fail(f"package coordinate {kind} version or target differs")
    if value["contains_cuda"] is not (kind != "npm_main"):
        fail(f"package coordinate {kind} has false CUDA-content metadata")
    path = package_dir / filename
    if path.is_symlink() or not path.is_file():
        fail(f"package must be a regular non-symlink file: {filename}")
    if require_sha(value["sha256"], f"package digest {kind}") != sha256(path):
        fail(f"package digest mismatch: {kind}")
    if kind == "cli_archive":
        validate_cli_archive(path, version)
    return filename


def validate_doctor_output(value: dict[str, Any], raw: bytes, policy: str, status: str, effective: object, reason: object) -> None:
    require_exact_keys(value, {"schema_version", "policy", "cuda_compiled", "status", "effective_device", "devices", "reason", "selected_uuid"}, "raw doctor output")
    expected = {
        "schema_version": "fathomdb.doctor.gpu.v1", "policy": policy, "cuda_compiled": True,
        "status": status, "effective_device": effective, "devices": [], "reason": reason,
        "selected_uuid": None,
    }
    if value != expected:
        fail("raw doctor output differs from the requested policy result")
    ordered = json.dumps(expected, ensure_ascii=True, separators=(",", ":")).encode("ascii") + b"\n"
    if raw != ordered:
        fail("raw doctor output does not use the product's canonical field order")


def validate_cli_smoke(root: Path, name: str, value: dict[str, Any], version: str, archive_name: str, archive_sha: str) -> None:
    unavailable = name.startswith("forced-cuda-unavailable")
    policy = "cuda:0" if unavailable else "auto"
    status = "cuda_unavailable"
    effective: object = None if unavailable else "cpu"
    reason: object = "no_visible_cuda_device"
    exit_code = 65 if unavailable else 0
    stdout_name = name.removesuffix(".json") + "-stdout.json"
    require_exact_keys(value, {
        "schema_version", "consumer", "archive_filename", "archive_sha256", "target", "argv",
        "exit_code", "doctor_output_filename", "doctor_output_sha256", "status", "effective_device",
        "reason", "requested_policy", "requested_ordinal", "environment", "isolation", "evidence_provenance",
    }, f"CLI smoke {name}")
    expected_argv = [f"/tmp/fathomdb-cli/fathomdb-{version}-{TARGET}/fathomdb", "doctor", "gpu", "--json"]
    if value["schema_version"] != "fathomdb.cuda-package-cli-smoke/v2" or value["consumer"] != "cli":
        fail(f"CLI smoke {name} schema or consumer differs")
    if value["archive_filename"] != archive_name or value["archive_sha256"] != archive_sha or value["target"] != TARGET:
        fail(f"CLI smoke {name} does not bind the retained archive")
    if value["argv"] != expected_argv or value["requested_policy"] != policy or value["requested_ordinal"] != (0 if unavailable else None):
        fail(f"CLI smoke {name} request does not bind policy and ordinal")
    expected_environment = {"FATHOMDB_EMBED_DEVICE": policy} if unavailable else {}
    if value["environment"] != expected_environment:
        fail(f"CLI smoke {name} environment is not exact")
    if value["isolation"] != {"database_opened": False, "model_loaded": False, "network": "none", "source_checkout_mounted": False}:
        fail(f"CLI smoke {name} isolation is not diagnostic-only")
    if value["evidence_provenance"] != "installed_candidate" or value["exit_code"] != exit_code or value["status"] != status or value["effective_device"] != effective or value["reason"] != reason:
        fail(f"CLI smoke {name} outcome is not truthful")
    if value["doctor_output_filename"] != stdout_name:
        fail(f"CLI smoke {name} names the wrong raw output")
    output_path = root / SMOKE_DIR / stdout_name
    if output_path.is_symlink():
        fail(f"CLI raw output {stdout_name} must not be a symlink")
    try:
        raw = output_path.read_bytes()
        output = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read CLI raw output {stdout_name}: {error}")
    if not isinstance(output, dict):
        fail(f"CLI raw output {stdout_name} must be an object")
    if require_sha(value["doctor_output_sha256"], f"CLI output digest {name}") != hashlib.sha256(raw).hexdigest():
        fail(f"CLI smoke {name} raw output digest differs")
    validate_doctor_output(output, raw, policy, status, effective, reason)


def validate_reranker_cli_doctor(root: Path, value: dict[str, Any], version: str, archive_name: str, archive_sha: str) -> None:
    require_exact_keys(value, {
        "schema_version", "consumer", "archive_filename", "archive_sha256", "target", "argv",
        "requested_policy", "environment", "isolation", "evidence_provenance", "exit_code",
        "doctor_output_filename", "doctor_output_sha256", "effective_device", "reason",
    }, "reranker CLI doctor")
    expected_argv = [f"/tmp/fathomdb-cli/fathomdb-{version}-{TARGET}/fathomdb", "doctor", "reranker-gpu", "--json"]
    if (
        value["schema_version"] != "fathomdb.cuda-reranker-cli-doctor/v1"
        or value["consumer"] != "cli"
        or value["archive_filename"] != archive_name
        or value["archive_sha256"] != archive_sha
        or value["target"] != TARGET
        or value["argv"] != expected_argv
        or value["requested_policy"] != "auto"
        or value["environment"] != {}
        or value["isolation"] != {"database_opened": False, "model_loaded": False, "network": "none", "source_checkout_mounted": False}
        or value["evidence_provenance"] != "installed_candidate"
        or value["exit_code"] != 0
        or value["doctor_output_filename"] != "reranker-cli-doctor-stdout.json"
        or value["effective_device"] != "cpu"
        or value["reason"] != "no_visible_cuda_device"
    ):
        fail("reranker CLI doctor does not prove the isolated CPU diagnostic contract")
    output_path = root / SMOKE_DIR / "reranker-cli-doctor-stdout.json"
    if output_path.is_symlink() or not output_path.is_file():
        fail("reranker CLI doctor raw output must be a regular non-symlink file")
    raw = output_path.read_bytes()
    if require_sha(value["doctor_output_sha256"], "reranker CLI doctor output digest") != hashlib.sha256(raw).hexdigest():
        fail("reranker CLI doctor raw output digest differs")
    try:
        output = json.loads(raw)
    except json.JSONDecodeError as error:
        fail(f"cannot read reranker CLI doctor raw output: {error}")
    expected = {
        "schema_version": "fathomdb.doctor.reranker-gpu.v1", "subsystem": "reranker",
        "policy": "auto", "cuda_compiled": True, "effective_device": "cpu", "devices": [],
        "reason": "no_visible_cuda_device", "selected_uuid": None,
    }
    if raw != json.dumps(expected, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n" or output != expected:
        fail("reranker CLI doctor raw output differs from the product CPU diagnostic")


def validate_smokes(root: Path, expected: object, version: str, archive_name: str, archive_sha: str, rerank: bool) -> None:
    smoke_names = BASE_SMOKE_NAMES | (RERANK_V3_SMOKE_NAMES if rerank else frozenset())
    if not isinstance(expected, dict) or set(expected) != smoke_names:
        fail("smoke evidence inventory is incomplete or contains unknown evidence")
    smoke_dir = root / SMOKE_DIR
    require_regular_directory(smoke_dir, "smoke evidence directory")
    actual_names = {path.name for path in smoke_dir.iterdir()}
    if actual_names != smoke_names:
        fail("smoke evidence directory does not exactly match manifest inventory")
    for name in sorted(smoke_names):
        path = smoke_dir / name
        if path.is_symlink() or not path.is_file():
            fail(f"smoke evidence must be a regular file: {name}")
        if require_sha(expected[name], f"smoke evidence digest {name}") != sha256(path):
            fail(f"smoke evidence digest mismatch: {name}")
        if name.endswith("-stdout.json"):
            continue
        value, _ = load_object(path, f"smoke evidence {name}")
        if name in {"cpu-cli.json", "forced-cuda-unavailable-cli.json"}:
            validate_cli_smoke(root, name, value, version, archive_name, archive_sha)
            continue
        if name == "reranker-cli-doctor.json":
            validate_reranker_cli_doctor(root, value, version, archive_name, archive_sha)
            continue
        kind, consumer = name.removesuffix(".json").split("-", 1)
        if kind == "cpu":
            validate_cpu_smoke(value, consumer)
        elif kind == "gpu":
            validate_gpu_smoke(value, consumer)


def validate_future_reranker_gpu_receipt(path: Path, candidate_sha: str) -> None:
    value, _ = load_object(path, "future reranker GPU inference receipt")
    require_exact_keys(value, {
        "schema_version", "candidate_sha", "consumer", "target", "requested_policy", "status",
        "effective_device", "visible_devices", "selected_uuid", "nvidia_smi_uuid", "process_id",
        "nvidia_smi_compute_process_id", "model_cache_manifest_sha256", "rerank_performed", "network",
        "source_imported",
    }, "future reranker GPU inference receipt")
    if (
        value["schema_version"] != "fathomdb.cuda-reranker-gpu-inference-receipt/v1"
        or require_sha(value["candidate_sha"], "future reranker receipt candidate SHA", COMMIT_SHA) != candidate_sha
        or value["consumer"] != "cli"
        or value["target"] != TARGET
        or value["requested_policy"] != "cuda:0"
        or value["status"] != "selected_cuda"
        or value["effective_device"] != "cuda:0"
        or value["rerank_performed"] is not True
        or value["network"] != "none"
        or value["source_imported"] is not False
    ):
        fail("future reranker GPU receipt identity or inference outcome differs")
    require_sha(value["model_cache_manifest_sha256"], "future reranker receipt cache manifest digest")
    devices = value["visible_devices"]
    if not isinstance(devices, list) or len(devices) != 1 or not isinstance(devices[0], dict):
        fail("future reranker GPU receipt must retain exactly one selected visible device")
    device = devices[0]
    require_exact_keys(device, {"visible_ordinal", "uuid", "name", "compute_capability"}, "future reranker selected device")
    if device["visible_ordinal"] != 0 or not all(isinstance(device[name], str) and device[name] for name in ("uuid", "name", "compute_capability")):
        fail("future reranker selected device is malformed")
    if value["selected_uuid"] != device["uuid"] or value["nvidia_smi_uuid"] != device["uuid"]:
        fail("future reranker GPU receipt lacks UUID correlation")
    if (
        not isinstance(value["process_id"], int) or isinstance(value["process_id"], bool) or value["process_id"] < 1
        or value["nvidia_smi_compute_process_id"] != value["process_id"]
    ):
        fail("future reranker GPU receipt lacks PID correlation")


def validate(root: Path, candidate_sha: str) -> None:
    if require_sha(candidate_sha, "requested candidate SHA", COMMIT_SHA) != candidate_sha:
        fail("unreachable")
    require_regular_directory(root, "rehearsal directory")
    allowed_root = {MANIFEST, ROUTE_RECEIPT, PREFLIGHT_WITNESS_DIR, BUILD_INPUT, PACKAGE_DIR, SMOKE_DIR}
    if {path.name for path in root.iterdir()} != allowed_root:
        fail("rehearsal directory has missing or unknown members")
    manifest, _ = load_object(root / MANIFEST, "rehearsal manifest")
    require_exact_keys(
        manifest,
        {"schema_version", "candidate_sha", "version", "target", "route_receipt_sha256", "preflight_witness_sha256", "build_input", "packages", "smoke_evidence_sha256", "pending_external"},
        "rehearsal manifest",
    )
    if require_sha(manifest["candidate_sha"], "rehearsal candidate SHA", COMMIT_SHA) != candidate_sha:
        fail("rehearsal manifest candidate SHA does not match the requested candidate")
    version = manifest["version"]
    if not isinstance(version, str) or re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) is None or manifest["target"] != TARGET:
        fail("rehearsal version or target is invalid")
    route, route_bytes = load_object(root / ROUTE_RECEIPT, "route receipt")
    if route.get("schema_version") == MAIN_ROUTE_SCHEMA:
        require_exact_keys(
            route,
            {"schema_version", "workflow_ref", "workflow_sha", "run_id", "run_attempt", "candidate_sha"},
            "main-route receipt",
        )
        if route["workflow_ref"] != MAIN_WORKFLOW_REF:
            fail("main-route receipt workflow ref is invalid")
        require_sha(route["workflow_sha"], "main-route receipt workflow SHA", COMMIT_SHA)
        for field in ("run_id", "run_attempt"):
            if not isinstance(route[field], int) or isinstance(route[field], bool) or route[field] < 1:
                fail(f"main-route receipt {field} is invalid")
    elif route.get("schema_version") != UNMERGED_ROUTE_SCHEMA:
        fail("route receipt schema is unsupported")
    if route.get("candidate_sha") != candidate_sha:
        fail("route receipt does not bind the requested candidate")
    if require_sha(manifest["route_receipt_sha256"], "route receipt digest") != hashlib.sha256(route_bytes).hexdigest():
        fail("route receipt digest mismatch")
    preflight_witness_dir = root / PREFLIGHT_WITNESS_DIR
    require_regular_directory(preflight_witness_dir, "preflight witness directory")
    preflight_witness, preflight_witness_bytes = load_preflight_witness(
        preflight_witness_dir / PREFLIGHT_WITNESS,
    )
    if (
        preflight_witness.get("schema_version") not in {
            "fathomdb.cuda-preflight-witness/v2", "fathomdb.cuda-preflight-witness/v3"
        }
        or preflight_witness.get("candidate_sha") != candidate_sha
        or preflight_witness.get("outcome") != "passed"
    ):
        fail("preflight witness does not bind the requested candidate passed outcome")
    if require_sha(manifest["preflight_witness_sha256"], "preflight witness digest") != hashlib.sha256(preflight_witness_bytes).hexdigest():
        fail("preflight witness digest mismatch")
    preflight_verifier = Path(__file__).with_name("verify-cuda-preflight-witness.py")
    result = subprocess.run(
        [sys.executable, str(preflight_verifier), "--witness-dir", str(preflight_witness_dir), "--candidate-sha", candidate_sha],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown failure"
        fail(f"retained preflight witness fails Slice 10 validation: {detail}")

    build_input_path = root / BUILD_INPUT
    build_input, build_input_bytes = load_object(build_input_path, "build input")
    if manifest["build_input"] != build_input:
        fail("manifest build input differs from the retained build input")
    build_schema = validate_build_input(build_input, candidate_sha, version)
    expected_preflight_schema = (
        "fathomdb.cuda-preflight-witness/v3"
        if build_schema == BUILD_INPUT_SCHEMA_V3
        else "fathomdb.cuda-preflight-witness/v2"
    )
    if preflight_witness["schema_version"] != expected_preflight_schema:
        fail("preflight witness schema does not match the retained build input")
    expected_manifest_schema = SCHEMA_VERSION_V3 if build_schema == BUILD_INPUT_SCHEMA_V3 else SCHEMA_VERSION_V2
    if manifest["schema_version"] != expected_manifest_schema:
        fail("rehearsal manifest schema does not match the retained build input")
    expected_pending = (
        ["compatible_gpu_cli", "incompatible_classifier_observation"]
        if build_schema == BUILD_INPUT_SCHEMA_V2
        else ["compatible_gpu_reranker_cli", "incompatible_reranker_classifier_observation"]
    )
    if manifest["pending_external"] != expected_pending:
        fail("unavailable real CLI hardware evidence must remain PENDING_EXTERNAL")
    model_manifest = preflight_witness_dir / "model-cache-manifest.json"
    if build_input["model_cache_manifest_sha256"] != sha256(model_manifest):
        fail("build input does not bind the retained Slice 10 model cache manifest")
    _ = build_input_bytes

    packages = manifest["packages"]
    if not isinstance(packages, dict) or set(packages) != PACKAGE_KINDS:
        fail("package inventory must contain exactly four typed retained artifacts")
    package_dir = root / PACKAGE_DIR
    require_regular_directory(package_dir, "package directory")
    filenames = {
        validate_package_coordinate(kind, packages[kind], version, package_dir)
        for kind in sorted(PACKAGE_KINDS)
    }
    if {path.name for path in package_dir.iterdir()} != filenames:
        fail("package directory does not exactly match manifest inventory")
    archive = packages["cli_archive"]
    validate_smokes(
        root, manifest["smoke_evidence_sha256"], version, archive["filename"], archive["sha256"],
        build_schema == BUILD_INPUT_SCHEMA_V3,
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rehearsal-dir", required=True, type=Path)
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument(
        "--future-reranker-gpu-receipt", type=Path,
        help="validate one external installed-CLI CUDA reranker inference receipt without promoting it",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    validate(args.rehearsal_dir, args.candidate_sha)
    if args.future_reranker_gpu_receipt is not None:
        validate_future_reranker_gpu_receipt(args.future_reranker_gpu_receipt, args.candidate_sha)
    print("cuda-package-rehearsal: pass")


if __name__ == "__main__":
    main()
