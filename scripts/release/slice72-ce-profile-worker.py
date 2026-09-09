#!/usr/bin/env python3
"""Run one installed-artifact CE profile process."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import resource
import subprocess
import sys
import tempfile
import time
from typing import Any


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def output_digest(output: dict[str, Any]) -> str:
    payload = json.dumps(output, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


def sample_cuda_process(pid: int, samples: list[dict[str, Any]]) -> None:
    result = subprocess.run(
        [
            "nvidia-smi",
            "--query-compute-apps=gpu_uuid,pid,used_memory",
            "--format=csv,noheader,nounits",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if result.returncode == 0:
        for line in result.stdout.splitlines():
            parts = [part.strip() for part in line.split(",")]
            if len(parts) == 3 and parts[1] == str(pid):
                try:
                    samples.append(
                        {"gpu_uuid": parts[0], "pid": pid, "vram_mib": float(parts[2])}
                    )
                except ValueError:
                    pass


def output_from_standalone(fathomdb: Any, fixture: dict[str, Any]) -> dict[str, Any]:
    result = fathomdb.rerank(
        fixture["query"],
        [dict(item) for item in fixture["passages"]],
        fixture["rerank_depth"],
        alpha=fixture["alpha"],
        pool_n=fixture["pool_n"],
    )
    return {
        "ids": [int(item["id"]) for item in result],
        "ce_scores": {str(item["id"]): item["ce_score"] for item in result},
    }


def output_from_engine(engine: Any, fixture: dict[str, Any]) -> dict[str, Any]:
    result = engine.search(
        fixture["query"],
        rerank_depth=fixture["rerank_depth"],
        alpha=fixture["alpha"],
        pool_n=fixture["pool_n"],
        limit=len(fixture["passages"]),
    )
    ids: list[int] = []
    scores: dict[str, float | None] = {}
    for hit in result.results:
        if not hit.source_id or not hit.source_id.startswith("slice72:"):
            raise RuntimeError(f"unexpected Engine source_id: {hit.source_id!r}")
        item = int(hit.source_id.split(":", 1)[1])
        ids.append(item)
        scores[str(item)] = hit.ce_score
    return {"ids": ids, "ce_scores": scores}


def validate_runtime_output(output: dict[str, Any]) -> None:
    scores = output["ce_scores"]
    if not scores or any(
        value is None or not isinstance(value, (int, float)) or not math.isfinite(value)
        for value in scores.values()
    ):
        raise RuntimeError("CE did not produce finite scores for every returned hit")
    if len(set(float(value) for value in scores.values())) < 2:
        raise RuntimeError("CE scores are degenerate")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--path", choices=["standalone", "engine"], required=True)
    parser.add_argument("--mode", choices=["cold", "steady"], required=True)
    parser.add_argument("--database-dir", type=Path, required=True)
    args = parser.parse_args()

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    os.sched_setaffinity(0, set(manifest["cpu_affinity"]))
    started = time.perf_counter_ns()
    import fathomdb
    import fathomdb._fathomdb as native

    import_ns = time.perf_counter_ns() - started
    install_root = Path(sys.prefix).resolve()
    module_path = Path(fathomdb.__file__).resolve()
    native_path = Path(native.__file__).resolve()
    native_sha256 = sha256(native_path)
    source_imported = not (
        module_path.is_relative_to(install_root) and native_path.is_relative_to(install_root)
    )
    if source_imported:
        raise RuntimeError(
            f"installed profile imported outside {install_root}: {module_path}, {native_path}"
        )

    args.database_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        prefix="slice72-", suffix=".fdb", dir=args.database_dir, delete=False
    ) as database:
        database_path = database.name
    Path(database_path).unlink()
    open_started = time.perf_counter_ns()
    engine = fathomdb.Engine.open(database_path, use_default_embedder=False)
    open_ns = time.perf_counter_ns() - open_started
    resolution = engine.open_report().reranker_device_resolution
    if resolution is None:
        raise RuntimeError("installed artifact did not report reranker device resolution")
    kind = resolution.effective_device.kind
    effective_device = "cuda:0" if kind == "cuda" else kind
    selected_uuid = resolution.selected_cuda_uuid

    fixture = manifest["fixture"]
    if args.path == "engine":
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": item["body"],
                    "source_id": f"slice72:{item['id']}",
                }
                for item in fixture["passages"]
            ]
        )
        engine.drain(timeout_s=5)

        def operation() -> dict[str, Any]:
            return output_from_engine(engine, fixture)

    else:

        def operation() -> dict[str, Any]:
            return output_from_standalone(fathomdb, fixture)

    samples: list[dict[str, Any]] = []
    if args.mode == "cold":
        call_started = time.perf_counter_ns()
        output = operation()
        duration_ns: int | list[int] = time.perf_counter_ns() - call_started
        output_digests = [output_digest(output)]
    else:
        reference = operation()
        validate_runtime_output(reference)
        reference_digest = output_digest(reference)
        durations = []
        output_digests = []
        for _ in range(manifest["steady_calls"]):
            call_started = time.perf_counter_ns()
            current = operation()
            durations.append(time.perf_counter_ns() - call_started)
            current_digest = output_digest(current)
            if current_digest != reference_digest or current != reference:
                raise RuntimeError("CE output changed between steady calls")
            output_digests.append(current_digest)
        output = reference
        duration_ns = durations
    validate_runtime_output(output)
    if effective_device == "cuda:0":
        sample_cuda_process(os.getpid(), samples)

    allocation = None
    if samples:
        allocation = max(samples, key=lambda item: item["vram_mib"])
    result: dict[str, Any] = {
        "schema_version": "fathomdb.slice72.ce-profile-process/v1",
        "path": args.path,
        "mode": args.mode,
        "pid": os.getpid(),
        "install_root": str(install_root),
        "module_path": str(module_path),
        "native_path": str(native_path),
        "native_sha256": native_sha256,
        "source_imported": source_imported,
        "effective_device": effective_device,
        "selected_uuid": selected_uuid,
        "allocation": allocation,
        "import_ns": import_ns,
        "open_ns": open_ns,
        "peak_rss_kib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
        "affinity": sorted(os.sched_getaffinity(0)),
        "output_digests": output_digests,
        "output": output,
    }
    if args.mode == "cold":
        result["duration_ns"] = duration_ns
    else:
        result["durations_ns"] = duration_ns
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    engine.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
