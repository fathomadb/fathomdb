#!/usr/bin/env python3
"""Fail-closed verifier for retained CUDA embedding v2 and reranker v3 witnesses."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, NoReturn


SCHEMA_VERSION_V2 = "fathomdb.cuda-preflight-witness/v2"
SCHEMA_VERSION_V3 = "fathomdb.cuda-preflight-witness/v3"
WITNESS_NAME = "cuda-preflight-witness.json"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
COMMIT_SHA = re.compile(r"[0-9a-f]{40}\Z")
COMPUTE_CAPABILITY = re.compile(r"[0-9]+\.[0-9]+\Z")
REPOSITORY = "BAAI/bge-small-en-v1.5"
REVISION = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"
MODEL_DIGESTS = {
    "config.json": "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750",
    "tokenizer.json": "".join((
        "d241a60d5e8f04cc1", "b2b3e9ef7a4921b2", "7bf526d9f6050ab9", "0f9267a1f9e5c66",
    )),
    "model.safetensors": "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad",
}
EXPECTED_FORCED_MESSAGES = {
    "no_visible_cuda_device": "cuda:0 requested but unavailable: NoVisibleCudaDevice",
    "cuda_incompatible": "cuda:0 requested but unavailable: CudaIncompatible",
}
EVIDENCE_NAMES = frozenset({
    "environment.txt",
    "manylinux-build.txt",
    "dynamic-dependencies.txt",
    "python-auditwheel.txt",
    "driverless-python-cpu-smoke.txt",
    "driverless-napi-cpu-smoke.txt",
    "gpu-python-cuda-witness.json",
    "gpu-napi-cuda-witness.json",
    "gpu-python-cuda-smoke.txt",
    "gpu-napi-cuda-smoke.txt",
    "build-input.json",
    "model-cache-manifest.json",
    "smoke-cache-topology.json",
    "forced-python-open.py",
    "forced-napi-open.mjs",
    "forced-cuda-unavailable-python.json",
    "forced-cuda-unavailable-napi.json",
    "forced-cuda-unavailable-python-stdout.txt",
    "forced-cuda-unavailable-python-stderr.txt",
    "forced-cuda-unavailable-napi-stdout.txt",
    "forced-cuda-unavailable-napi-stderr.txt",
})


def fail(message: str) -> NoReturn:
    print(f"cuda-preflight-witness: {message}", file=sys.stderr)
    raise SystemExit(1)


def canonical_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def require_exact_keys(value: dict[str, Any], expected: set[str] | frozenset[str], label: str) -> None:
    if set(value) != expected:
        fail(f"{label} has missing or unknown fields")


def require_string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        fail(f"{label} must be a nonempty string")
    return value


def require_digest(value: object, label: str) -> str:
    if not isinstance(value, str) or SHA256.fullmatch(value) is None:
        fail(f"{label} must be a lowercase SHA-256")
    return value


def load_canonical_object(path: Path, label: str) -> tuple[dict[str, Any], bytes]:
    if path.is_symlink() or not path.is_file():
        fail(f"{label} must be a regular non-symlink file")
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


def validate_device(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(f"{label} must be an object")
    require_exact_keys(value, {"visible_ordinal", "uuid", "name", "compute_capability"}, label)
    if value["visible_ordinal"] != 0:
        fail(f"{label} must describe visible ordinal zero")
    require_string(value["uuid"], f"{label} UUID")
    require_string(value["name"], f"{label} name")
    capability = value["compute_capability"]
    if not isinstance(capability, str) or COMPUTE_CAPABILITY.fullmatch(capability) is None:
        fail(f"{label} compute capability is invalid")
    return value


def validate_capture(
    value: dict[str, Any], consumer: str, status: str, reason: str, stderr: bytes,
) -> None:
    require_exact_keys(value, {
        "schema_version", "consumer", "argv", "requested_policy", "status",
        "effective_device", "reason", "error",
    }, f"forced {consumer} capture")
    expected_argv = (
        ["/opt/python/cp311-cp311/bin/python", "/fathomdb-harness/forced-python-open.py"]
        if consumer == "python"
        else ["node", "/fathomdb-harness/forced-napi-open.mjs"]
    )
    if value["schema_version"] != "fathomdb.cuda-forced-device-capture/v1":
        fail(f"forced {consumer} capture schema is unsupported")
    if value["consumer"] != consumer or value["argv"] != expected_argv:
        fail(f"forced {consumer} capture identity or argv differs")
    if (
        value["requested_policy"] != "cuda:0"
        or value["status"] != status
        or value["effective_device"] is not None
        or value["reason"] != reason
    ):
        fail(f"forced {consumer} capture semantic outcome differs")
    error = value["error"]
    if not isinstance(error, dict):
        fail(f"forced {consumer} capture lacks a typed error")
    require_exact_keys(error, {"type", "kind", "ordinal", "message"}, f"forced {consumer} typed error")
    expected_message = EXPECTED_FORCED_MESSAGES.get(reason)
    if expected_message is None:
        fail(f"forced {consumer} capture reason has no stable typed message")
    if error != {
        "type": "EmbedDevicePolicyError", "kind": reason, "ordinal": 0, "message": expected_message,
    }:
        fail(f"forced {consumer} capture typed error does not match its stable reason message")
    if stderr != (expected_message + "\n").encode("utf-8"):
        fail(f"forced {consumer} stderr does not equal its exact typed error")


def validate_forced_record(
    record_path: Path,
    stdout_path: Path,
    stderr_path: Path,
    *,
    expected_consumer: str | None = None,
    fixture: bool = False,
) -> None:
    record, _ = load_canonical_object(record_path, "forced-device record")
    require_exact_keys(record, {
        "schema_version", "consumer", "requested_policy", "cuda_compiled", "visible_devices",
        "status", "effective_device", "reason", "provenance", "command", "exit_code",
        "stdout_filename", "stdout_sha256", "stderr_filename", "stderr_sha256",
    }, "forced-device record")
    consumer = record["consumer"]
    if consumer not in {"python", "napi"} or (expected_consumer is not None and consumer != expected_consumer):
        fail("forced-device record consumer differs")
    if record["schema_version"] != "fathomdb.cuda-forced-device-failure/v1":
        fail("forced-device record schema is unsupported")
    if record["requested_policy"] != "cuda:0" or record["cuda_compiled"] is not True:
        fail("forced-device record does not prove compiled forced cuda:0")
    if record["effective_device"] is not None or record["exit_code"] != 1:
        fail("forced-device failure selected a device or succeeded")

    if fixture:
        status = reason = "cuda_incompatible"
        if record["provenance"] != "deterministic_fixture_provider":
            fail("incompatible record lacks deterministic fixture provenance")
        if record["command"] != "deterministic_fixture_provider":
            fail("incompatible fixture record has an installed-candidate command")
        devices = record["visible_devices"]
        if not isinstance(devices, list) or len(devices) != 1:
            fail("incompatible fixture must contain exactly one visible device")
        validate_device(devices[0], "incompatible fixture device")
    else:
        status, reason = "cuda_unavailable", "no_visible_cuda_device"
        if record["provenance"] != "installed_candidate":
            fail("sealed forced-device record is not installed-candidate evidence")
        if record["command"] != f"installed_{consumer}_engine_open":
            fail("sealed forced-device record command differs")
        if record["visible_devices"] != []:
            fail("unavailable installed-candidate record must have empty inventory")
    if record["status"] != status or record["reason"] != reason:
        fail("forced-device record classification differs")
    if record["stdout_filename"] != stdout_path.name or record["stderr_filename"] != stderr_path.name:
        fail("forced-device record names the wrong capture")
    for path, field, label in (
        (stdout_path, "stdout_sha256", "forced stdout"),
        (stderr_path, "stderr_sha256", "forced stderr"),
    ):
        if path.is_symlink() or not path.is_file():
            fail(f"{label} must be a regular non-symlink file")
        raw = path.read_bytes()
        if not raw or require_digest(record[field], field) != sha256_bytes(raw):
            fail(f"{label} digest mismatch")
    capture, _ = load_canonical_object(stdout_path, f"forced {consumer} stdout capture")
    validate_capture(capture, consumer, status, reason, stderr_path.read_bytes())


def validate_gpu_observation(path: Path, consumer: str) -> None:
    value, _ = load_canonical_object(path, f"GPU {consumer} observation")
    require_exact_keys(value, {
        "schema_version", "consumer", "requested_policy", "status", "effective_device",
        "visible_devices", "selected_uuid", "nvidia_smi_uuid", "process_id",
        "nvidia_smi_compute_process_id", "process_name",
    }, f"GPU {consumer} observation")
    if (
        value["schema_version"] != "fathomdb.cuda-device-observation/v1"
        or value["consumer"] != consumer
        or value["requested_policy"] != "cuda:0"
        or value["status"] != "selected_cuda"
        or value["effective_device"] != "cuda:0"
    ):
        fail(f"GPU {consumer} observation identity or selection differs")
    devices = value["visible_devices"]
    if not isinstance(devices, list) or len(devices) != 1:
        fail(f"GPU {consumer} observation must contain exactly one visible device")
    device = validate_device(devices[0], f"GPU {consumer} selected device")
    if value["selected_uuid"] != device["uuid"] or value["nvidia_smi_uuid"] != device["uuid"]:
        fail(f"GPU {consumer} observation lacks UUID correlation")
    process_id = value["process_id"]
    smi_process_id = value["nvidia_smi_compute_process_id"]
    if (
        not isinstance(process_id, int) or isinstance(process_id, bool) or process_id < 1
        or not isinstance(smi_process_id, int) or isinstance(smi_process_id, bool)
        or smi_process_id != process_id
    ):
        fail(f"GPU {consumer} observation lacks PID correlation")
    require_string(value["process_name"], f"GPU {consumer} process name")


def validate_model_manifest(path: Path) -> tuple[dict[str, Any], bytes]:
    value, raw = load_canonical_object(path, "model-cache manifest")
    require_exact_keys(value, {"schema_version", "repository", "revision", "snapshot_relpath", "files"}, "model-cache manifest")
    expected = {
        "schema_version": "fathomdb.cuda-model-cache/v1",
        "repository": REPOSITORY,
        "revision": REVISION,
        "snapshot_relpath": f"hub/models--BAAI--bge-small-en-v1.5/snapshots/{REVISION}",
        "files": MODEL_DIGESTS,
    }
    if value != expected:
        fail("model-cache manifest differs from the fixed artifact contract")
    return value, raw


def validate_build_input(path: Path, candidate_sha: str, model_digest: str) -> str:
    value, raw = load_canonical_object(path, "preflight build input")
    expected = {
        "schema_version": "fathomdb.cuda-preflight-build-input/v2",
        "candidate_sha": candidate_sha,
        "target": "x86_64-unknown-linux-gnu",
        "python_features": ["embed-cuda", "pyo3/extension-module"],
        "napi_features": ["default-embedder", "embed-cuda"],
        "rerank_cuda": False,
        "model_cache_manifest_sha256": model_digest,
    }
    require_exact_keys(value, set(expected), "preflight build input")
    if value["schema_version"] == "fathomdb.cuda-preflight-build-input/v3":
        expected.update({
            "schema_version": "fathomdb.cuda-preflight-build-input/v3",
            "python_features": ["embed-cuda", "rerank-cuda", "pyo3/extension-module"],
            "napi_features": ["default-embedder", "embed-cuda", "rerank-cuda"],
            "rerank_cuda": True,
        })
    if value != expected:
        fail("preflight build input differs from the fixed build contract")
    return "v3" if value["rerank_cuda"] else "v2"


def validate_cache_topology(path: Path, manifest: dict[str, Any]) -> None:
    value, _ = load_canonical_object(path, "smoke-cache topology")
    require_exact_keys(value, {"schema_version", "smokes"}, "smoke-cache topology")
    if value["schema_version"] != "fathomdb.cuda-smoke-cache-topology/v1":
        fail("smoke-cache topology schema is unsupported")
    smokes = value["smokes"]
    expected_smokes = {"driverless_python", "driverless_napi", "gpu_python", "gpu_napi"}
    if not isinstance(smokes, dict) or set(smokes) != expected_smokes:
        fail("smoke-cache topology inventory differs")
    prefix = sha256_bytes(f"{manifest['repository']}@{manifest['revision']}".encode("utf-8"))[:12]
    expected_files = {
        f"fathomdb/embedders/{prefix}/{name}": digest for name, digest in manifest["files"].items()
    }
    for name, smoke in smokes.items():
        if not isinstance(smoke, dict):
            fail(f"smoke-cache topology {name} is not an object")
        require_exact_keys(smoke, {
            "hf_home", "hf_seed_read_only", "xdg_cache_home", "network",
            "product_cache_initial_entries", "product_cache_files",
        }, f"smoke-cache topology {name}")
        if smoke != {
            "hf_home": "/fathomdb-hf",
            "hf_seed_read_only": True,
            "xdg_cache_home": "/fathomdb-product-cache",
            "network": "none",
            "product_cache_initial_entries": 0,
            "product_cache_files": expected_files,
        }:
            fail(f"smoke-cache topology {name} does not prove isolated materialization")


def validate(witness_dir: Path, candidate_sha: str) -> None:
    if COMMIT_SHA.fullmatch(candidate_sha) is None:
        fail("requested candidate SHA must be a lowercase 40-hex commit")
    if witness_dir.is_symlink() or not witness_dir.is_dir():
        fail("witness directory must be a regular non-symlink directory")
    actual_names = {path.name for path in witness_dir.iterdir()}
    if actual_names != EVIDENCE_NAMES | {WITNESS_NAME}:
        fail("witness root inventory is incomplete or contains unknown members")
    witness, _ = load_canonical_object(witness_dir / WITNESS_NAME, "witness")
    require_exact_keys(witness, {
        "schema_version", "candidate_sha", "outcome", "build_input_sha256",
        "model_cache_manifest_sha256", "evidence_sha256",
    }, "witness")
    if witness["candidate_sha"] != candidate_sha:
        fail("witness candidate SHA does not match the requested candidate")
    if witness["outcome"] != "passed":
        fail("witness outcome is not passed")
    evidence = witness["evidence_sha256"]
    if not isinstance(evidence, dict) or set(evidence) != EVIDENCE_NAMES:
        fail("witness evidence inventory is incomplete or contains unknown evidence")
    for name in sorted(EVIDENCE_NAMES):
        path = witness_dir / name
        if path.is_symlink() or not path.is_file():
            fail(f"required evidence must be a regular non-symlink file: {name}")
        raw = path.read_bytes()
        if not raw:
            fail(f"required evidence is empty: {name}")
        if require_digest(evidence[name], f"evidence digest {name}") != sha256_bytes(raw):
            fail(f"evidence digest mismatch: {name}")
    model_manifest, model_raw = validate_model_manifest(witness_dir / "model-cache-manifest.json")
    model_digest = sha256_bytes(model_raw)
    if require_digest(witness["model_cache_manifest_sha256"], "root model-cache digest") != model_digest:
        fail("root model-cache digest differs from retained manifest")
    build_kind = validate_build_input(witness_dir / "build-input.json", candidate_sha, model_digest)
    expected_schema = SCHEMA_VERSION_V3 if build_kind == "v3" else SCHEMA_VERSION_V2
    if witness["schema_version"] != expected_schema:
        fail("witness schema version does not match the retained build input")
    build_raw = (witness_dir / "build-input.json").read_bytes()
    if require_digest(witness["build_input_sha256"], "root build-input digest") != sha256_bytes(build_raw):
        fail("root build-input digest differs from retained input")
    validate_cache_topology(witness_dir / "smoke-cache-topology.json", model_manifest)
    source_dir = Path(__file__).resolve().parent
    for name in ("forced-python-open.py", "forced-napi-open.mjs"):
        if (witness_dir / name).read_bytes() != (source_dir / name).read_bytes():
            fail(f"retained harness differs from the main-owned source: {name}")
    for consumer in ("python", "napi"):
        validate_forced_record(
            witness_dir / f"forced-cuda-unavailable-{consumer}.json",
            witness_dir / f"forced-cuda-unavailable-{consumer}-stdout.txt",
            witness_dir / f"forced-cuda-unavailable-{consumer}-stderr.txt",
            expected_consumer=consumer,
        )
        validate_gpu_observation(witness_dir / f"gpu-{consumer}-cuda-witness.json", consumer)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--witness-dir", required=True, type=Path)
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument(
        "--fixture-forced-record", nargs=3, metavar=("RECORD", "STDOUT", "STDERR"), type=Path,
        help="also validate one deterministic visible-incompatible fixture record",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    validate(args.witness_dir, args.candidate_sha)
    if args.fixture_forced_record is not None:
        validate_forced_record(*args.fixture_forced_record, fixture=True)
    print("cuda-preflight-witness: pass")


if __name__ == "__main__":
    main()
