#!/usr/bin/env python3
"""Run one source-independent CPU or CUDA Slice 72 CE profile cell."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Any

from slice72_ce_artifact import ArtifactError, sha256, validate_receipt


WORKER = Path(__file__).with_name("slice72-ce-profile-worker.py")
MODEL_FILES = ["config.json", "tokenizer.json", "model.safetensors"]
MODEL_DIR = Path("fathomdb/reranker/0290849b0459")


def hash_models(cache: Path) -> dict[str, str]:
    return {name: sha256(cache / MODEL_DIR / name) for name in MODEL_FILES}


def stage_model_cache(source: Path, destination: Path, expected: dict[str, str]) -> None:
    source_model = source / MODEL_DIR
    destination_model = destination / MODEL_DIR
    if destination.exists():
        raise RuntimeError(f"staged cache already exists: {destination}")
    destination_model.mkdir(parents=True)
    for name in MODEL_FILES:
        source_file = source_model / name
        if sha256(source_file) != expected[name]:
            raise RuntimeError(f"source model hash mismatch: {name}")
        target = destination_model / name
        shutil.copy2(source_file, target)
        target.chmod(0o444)
    (destination_model / ".lock").touch()
    if hash_models(destination) != expected:
        raise RuntimeError("staged model cache does not match the manifest")


def run_process(
    *,
    python: Path,
    manifest: Path,
    path: str,
    mode: str,
    raw_path: Path,
    database_dir: Path,
    environment: dict[str, str],
    affinity: list[int],
) -> dict[str, Any]:
    command = [
        "taskset", "-c", ",".join(str(cpu) for cpu in affinity),
        str(python), str(WORKER), "--manifest", str(manifest),
        "--path", path, "--mode", mode, "--database-dir", str(database_dir),
    ]
    result = subprocess.run(
        command, cwd=database_dir, env=environment, check=False, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    raw_path.write_text(
        json.dumps({"command": command, "exit_code": result.returncode,
                    "stdout": result.stdout, "stderr": result.stderr},
                   sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    if result.returncode:
        raise RuntimeError(f"profile worker failed; see {raw_path}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"profile worker emitted invalid JSON; see {raw_path}") from exc


def runtime_record(process: dict[str, Any]) -> dict[str, Any]:
    keys = [
        "pid", "install_root", "module_path", "native_path", "native_sha256",
        "source_imported", "effective_device", "selected_uuid", "allocation",
        "import_ns", "open_ns", "peak_rss_kib", "affinity",
    ]
    return {key: process[key] for key in keys}


def cuda_inventory(expected_uuid: str, expected_model: str) -> dict[str, str]:
    result = subprocess.run(
        [
            "nvidia-smi",
            "--query-gpu=uuid,name",
            "--format=csv,noheader",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode:
        raise RuntimeError("cannot inventory the selected CUDA device")
    matches = []
    for line in result.stdout.splitlines():
        uuid, separator, model = line.partition(",")
        if separator and uuid.strip() == expected_uuid:
            matches.append({"uuid": uuid.strip(), "model": model.strip()})
    if matches != [{"uuid": expected_uuid, "model": expected_model}]:
        raise RuntimeError("selected CUDA UUID/model is absent or ambiguous")
    return matches[0]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--artifact-receipt", type=Path, required=True)
    parser.add_argument("--cache-root", type=Path, required=True)
    parser.add_argument("--role", choices=["baseline", "candidate"], required=True)
    parser.add_argument("--device", choices=["cpu", "cuda"], required=True)
    parser.add_argument("--cuda-visible-devices")
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    manifest_path = args.manifest.resolve()
    artifact_path = args.artifact_receipt.resolve()
    source_cache = args.cache_root.resolve()
    output_dir = args.output_dir.resolve()
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    artifact = json.loads(artifact_path.read_text(encoding="utf-8"))
    validate_receipt(artifact, manifest, args.role, args.device)
    expected_cuda_selector = manifest["cuda"]["visible_selector"]
    if args.device == "cuda" and args.cuda_visible_devices != expected_cuda_selector:
        raise RuntimeError("CUDA cell selector must equal the manifest UUID selector")
    if args.device == "cpu" and args.cuda_visible_devices:
        raise RuntimeError("CPU cell must not set --cuda-visible-devices")

    output_dir.mkdir(parents=True, exist_ok=True)
    raw_dir = output_dir / "raw"
    database_dir = output_dir / "databases"
    staged_cache = output_dir / "model-cache"
    raw_dir.mkdir(exist_ok=True)
    database_dir.mkdir(exist_ok=True)
    stage_model_cache(source_cache, staged_cache, manifest["model_sha256"])
    initial_model_hashes = hash_models(staged_cache)

    excluded = {
        "PYTHONPATH", "PYTHONHOME", "FATHOMDB_RERANK_DEVICE", "HTTP_PROXY",
        "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY", "http_proxy", "https_proxy",
        "all_proxy", "no_proxy",
    }
    environment = {key: value for key, value in os.environ.items() if key not in excluded}
    environment.update(manifest["environment"])
    environment.update({
        "PYTHONNOUSERSITE": "1",
        "FATHOMDB_RERANKER_CACHE": str(staged_cache),
        "FATHOMDB_RERANK_DEVICE": "cpu" if args.device == "cpu" else "cuda:0",
        "HTTP_PROXY": "http://127.0.0.1:9",
        "HTTPS_PROXY": "http://127.0.0.1:9",
        "ALL_PROXY": "http://127.0.0.1:9",
        "NO_PROXY": "",
        "HF_HUB_OFFLINE": "1",
    })
    if args.device == "cuda":
        environment["CUDA_VISIBLE_DEVICES"] = args.cuda_visible_devices
        inventory = cuda_inventory(
            manifest["cuda"]["selected_uuid"], manifest["cuda"]["model"]
        )
    else:
        environment.pop("CUDA_VISIBLE_DEVICES", None)
        inventory = None

    python = Path(artifact["python_path"])
    paths: dict[str, Any] = {}
    process_number = 0
    for path in ["standalone", "engine"]:
        cold: list[dict[str, Any]] = []
        steady: list[dict[str, Any]] = []
        for mode, count, records in [
            ("cold", manifest["cold_repetitions"], cold),
            ("steady", manifest["steady_repetitions"], steady),
        ]:
            for index in range(count):
                process_number += 1
                raw_path = raw_dir / f"{path}-{mode}-{index + 1}.json"
                process = run_process(
                    python=python, manifest=manifest_path, path=path, mode=mode,
                    raw_path=raw_path,
                    database_dir=database_dir, environment=environment,
                    affinity=manifest["cpu_affinity"],
                )
                if hash_models(staged_cache) != initial_model_hashes:
                    raise RuntimeError(f"model cache changed after process {process_number}")
                record = {
                    "output": process["output"],
                    "output_digests": process["output_digests"],
                    "runtime": runtime_record(process),
                    "raw_sha256": sha256(raw_path),
                }
                if mode == "cold":
                    record["duration_ns"] = process["duration_ns"]
                else:
                    record["durations_ns"] = process["durations_ns"]
                records.append(record)
        paths[path] = {"cold": cold, "steady": steady}

    final_model_hashes = hash_models(staged_cache)
    cell = {
        "schema_version": "fathomdb.slice72.ce-profile-cell/v2",
        "role": args.role,
        "device": args.device,
        "commit_sha": artifact["commit_sha"],
        "features": artifact["features"],
        "artifact_receipt_sha256": sha256(artifact_path),
        "artifact": artifact,
        "network_policy": "offline-loopback-proxy",
        "cuda_inventory": inventory,
        "model_sha256_before": initial_model_hashes,
        "model_sha256_after": final_model_hashes,
        "model_files_read_only": all(
            (staged_cache / MODEL_DIR / name).stat().st_mode & 0o222 == 0
            for name in MODEL_FILES
        ),
        "paths": paths,
    }
    output = output_dir / f"{args.role}-{args.device}.json"
    output.write_text(json.dumps(cell, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ArtifactError, OSError, RuntimeError, KeyError, ValueError) as exc:
        print(f"FAIL Slice 72 CE profile runner: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
