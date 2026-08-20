"""Deterministic fixtures for the Slice 10 CUDA preflight-v2 verifier."""

from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path


CANDIDATE = "0123456789abcdef0123456789abcdef01234567"
OTHER_CANDIDATE = "89abcdef0123456789abcdef0123456789abcdef"
REPOSITORY = "BAAI/bge-small-en-v1.5"
REVISION = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"
MODEL_DIGESTS = {
    "config.json": "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750",
    "tokenizer.json": "".join((
        "d241a60d5e8f04cc1", "b2b3e9ef7a4921b2", "7bf526d9f6050ab9", "0f9267a1f9e5c66",
    )),
    "model.safetensors": "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad",
}
EVIDENCE_NAMES = {
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
}


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_json(path: Path, value: object) -> None:
    path.write_bytes(canonical(value))


def capture(consumer: str, status: str = "cuda_unavailable", reason: str = "no_visible_cuda_device") -> dict[str, object]:
    argv = (
        ["/opt/python/cp311-cp311/bin/python", "/fathomdb-harness/forced-python-open.py"]
        if consumer == "python"
        else ["node", "/fathomdb-harness/forced-napi-open.mjs"]
    )
    message = "cuda:0 requested but unavailable: NoVisibleCudaDevice"
    return {
        "schema_version": "fathomdb.cuda-forced-device-capture/v1",
        "consumer": consumer,
        "argv": argv,
        "requested_policy": "cuda:0",
        "status": status,
        "effective_device": None,
        "reason": reason,
        "error": {
            "type": "EmbedDevicePolicyError",
            "kind": reason,
            "ordinal": 0,
            "message": message,
        },
    }


def forced_record(
    root: Path,
    consumer: str,
    provenance: str = "installed_candidate",
    outcome: str = "unavailable",
) -> dict[str, object]:
    stdout_name = f"forced-cuda-{outcome}-{consumer}-stdout.txt"
    stderr_name = f"forced-cuda-{outcome}-{consumer}-stderr.txt"
    return {
        "schema_version": "fathomdb.cuda-forced-device-failure/v1",
        "consumer": consumer,
        "requested_policy": "cuda:0",
        "cuda_compiled": True,
        "visible_devices": [],
        "status": "cuda_unavailable",
        "effective_device": None,
        "reason": "no_visible_cuda_device",
        "provenance": provenance,
        "command": f"installed_{consumer}_engine_open",
        "exit_code": 1,
        "stdout_filename": stdout_name,
        "stdout_sha256": digest_bytes((root / stdout_name).read_bytes()),
        "stderr_filename": stderr_name,
        "stderr_sha256": digest_bytes((root / stderr_name).read_bytes()),
    }


def gpu_observation(consumer: str) -> dict[str, object]:
    uuid = "GPU-11111111-2222-3333-4444-555555555555"
    allocation_witness = {
        "schema": "fathomdb.tegra-gpu-allocation-witness/v1",
        "sole_gpu_consumer_precondition": "the witness run must be the sole GPU consumer: cuMemGetInfo reports a shared, system-wide counter on an integrated GPU",
        "device_ordinal_requested": 0,
        "device_ordinal_actual": 0,
        "device_uuid": uuid,
        "device_name": "NVIDIA fixture GPU",
        "compute_capability": "8.6",
        "free_before_bytes": 40_000_000_000,
        "free_after_bytes": 39_800_000_000,
        "total_bytes": 48_000_000_000,
        "delta_bytes": 200_000_000,
        "delta_floor_bytes": 67_108_864,
        "control_allocation_request_bytes": 268_435_456,
        "control_block_count": 1,
        "control_free_before_bytes": 42_000_000_000,
        "control_free_after_bytes": 41_700_000_000,
        "control_delta_bytes": 300_000_000,
        "embedded_vector_dim": 384,
    }
    return {
        "schema_version": "fathomdb.cuda-device-observation/v1",
        "consumer": consumer,
        "requested_policy": "cuda:0",
        "status": "selected_cuda",
        "effective_device": "cuda:0",
        "visible_devices": [{
            "visible_ordinal": 0,
            "uuid": uuid,
            "name": "NVIDIA fixture GPU",
            "compute_capability": "8.6",
        }],
        "selected_uuid": uuid,
        "nvidia_smi_uuid": uuid,
        "process_id": 4242,
        "nvidia_smi_compute_process_id": 4242,
        "process_name": "fixture-runtime",
        "allocation_witness": allocation_witness,
    }


def make_valid(root: Path, repo_root: Path) -> None:
    root.mkdir(parents=True, exist_ok=True)
    for name in (
        "environment.txt", "manylinux-build.txt", "dynamic-dependencies.txt", "python-auditwheel.txt",
        "driverless-python-cpu-smoke.txt", "driverless-napi-cpu-smoke.txt",
        "gpu-python-cuda-smoke.txt", "gpu-napi-cuda-smoke.txt",
    ):
        (root / name).write_text(f"fixture evidence: {name}\n", encoding="utf-8")
    shutil.copyfile(repo_root / "scripts/release/forced-python-open.py", root / "forced-python-open.py")
    shutil.copyfile(repo_root / "scripts/release/forced-napi-open.mjs", root / "forced-napi-open.mjs")
    for consumer in ("python", "napi"):
        stdout = canonical(capture(consumer))
        stderr = b"cuda:0 requested but unavailable: NoVisibleCudaDevice\n"
        (root / f"forced-cuda-unavailable-{consumer}-stdout.txt").write_bytes(stdout)
        (root / f"forced-cuda-unavailable-{consumer}-stderr.txt").write_bytes(stderr)
        write_json(root / f"forced-cuda-unavailable-{consumer}.json", forced_record(root, consumer))
        write_json(root / f"gpu-{consumer}-cuda-witness.json", gpu_observation(consumer))
    manifest = {
        "schema_version": "fathomdb.cuda-model-cache/v1",
        "repository": REPOSITORY,
        "revision": REVISION,
        "snapshot_relpath": f"hub/models--BAAI--bge-small-en-v1.5/snapshots/{REVISION}",
        "files": MODEL_DIGESTS,
    }
    write_json(root / "model-cache-manifest.json", manifest)
    model_manifest_digest = digest_bytes((root / "model-cache-manifest.json").read_bytes())
    build_input = {
        "schema_version": "fathomdb.cuda-preflight-build-input/v2",
        "candidate_sha": CANDIDATE,
        "target": "x86_64-unknown-linux-gnu",
        "python_features": ["embed-cuda", "pyo3/extension-module"],
        "napi_features": ["default-embedder", "embed-cuda"],
        "rerank_cuda": False,
        "model_cache_manifest_sha256": model_manifest_digest,
    }
    write_json(root / "build-input.json", build_input)
    prefix = digest_bytes(f"{REPOSITORY}@{REVISION}".encode())[:12]
    files = {f"fathomdb/embedders/{prefix}/{name}": value for name, value in MODEL_DIGESTS.items()}
    topology = {
        "schema_version": "fathomdb.cuda-smoke-cache-topology/v1",
        "smokes": {
            name: {
                "hf_home": "/fathomdb-hf",
                "hf_seed_read_only": True,
                "xdg_cache_home": "/fathomdb-product-cache",
                "network": "none",
                "product_cache_initial_entries": 0,
                "product_cache_files": files,
            }
            for name in ("driverless_python", "driverless_napi", "gpu_python", "gpu_napi")
        },
    }
    write_json(root / "smoke-cache-topology.json", topology)
    evidence = {name: digest_bytes((root / name).read_bytes()) for name in sorted(EVIDENCE_NAMES)}
    witness = {
        "schema_version": "fathomdb.cuda-preflight-witness/v2",
        "candidate_sha": CANDIDATE,
        "outcome": "passed",
        "build_input_sha256": evidence["build-input.json"],
        "model_cache_manifest_sha256": evidence["model-cache-manifest.json"],
        "evidence_sha256": evidence,
    }
    write_json(root / "cuda-preflight-witness.json", witness)


def make_incompatible_fixture(root: Path, consumer: str) -> tuple[Path, Path, Path]:
    root.mkdir(parents=True, exist_ok=True)
    stdout_name = f"forced-cuda-incompatible-{consumer}-stdout.txt"
    stderr_name = f"forced-cuda-incompatible-{consumer}-stderr.txt"
    value = capture(consumer, "cuda_incompatible", "cuda_incompatible")
    value["error"] = {
        "type": "EmbedDevicePolicyError",
        "kind": "cuda_incompatible",
        "ordinal": 0,
        "message": "cuda:0 requested but unavailable: CudaIncompatible",
    }
    (root / stdout_name).write_bytes(canonical(value))
    (root / stderr_name).write_text("cuda:0 requested but unavailable: CudaIncompatible\n", encoding="utf-8")
    record = forced_record(root, consumer, "deterministic_fixture_provider", "incompatible")
    record.update({
        "visible_devices": [{
            "visible_ordinal": 0,
            "uuid": "GPU-fixture-incompatible",
            "name": "NVIDIA incompatible fixture",
            "compute_capability": "3.0",
        }],
        "status": "cuda_incompatible",
        "reason": "cuda_incompatible",
        "command": "deterministic_fixture_provider",
        "stdout_filename": stdout_name,
        "stdout_sha256": digest_bytes((root / stdout_name).read_bytes()),
        "stderr_filename": stderr_name,
        "stderr_sha256": digest_bytes((root / stderr_name).read_bytes()),
    })
    record_path = root / f"forced-cuda-incompatible-{consumer}.json"
    write_json(record_path, record)
    return record_path, root / stdout_name, root / stderr_name
