#!/usr/bin/env python3
"""Installed Python worker for Slice130 lifecycle and instrumented soak requests."""

import argparse
from dataclasses import asdict
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import sys
import tempfile
import time

TEXT = "fathomdb sync allocator repair experiment"
PASSAGES = [
    {"id": 1, "body": "The Jetson AGX Orin shares DRAM between CPU and GPU.", "score": 0.5},
    {"id": 2, "body": "Stream-ordered allocation uses a per-device memory pool.", "score": 0.4},
]


def vector_hash(vector):
    """Validate the existing 384-dimensional finite-vector oracle and retain output."""
    if len(vector) != 384 or not all(math.isfinite(v) for v in vector):
        raise ValueError("invalid model embedding")
    return hashlib.sha256(struct.pack(f"<{len(vector)}d", *vector)).hexdigest()


def main():
    """Serve bounded supervisor requests; SDK imports always come from an owned venv."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lifecycle", action="store_true")
    args = parser.parse_args()
    import fathomdb
    from fathomdb import _native

    if sys.prefix == sys.base_prefix or not Path(fathomdb.__file__).resolve().is_relative_to(Path(sys.prefix).resolve()):
        raise RuntimeError("Python consumer requires an installed package in its owned venv")
    for policy in ("FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE"):
        if os.environ.get(policy) != "cuda:0":
            raise RuntimeError(f"{policy}=cuda:0 must be fixed before import")
    engines = []
    expected = []
    iteration = 0
    expected_scores = None

    def open_engine():
        directory = tempfile.mkdtemp(prefix="engine-", dir=os.environ["FATHOMDB_DB_SCRATCH"])
        engine = fathomdb.Engine.open(str(Path(directory) / "db.fdb"), use_default_embedder=True)
        resolution = engine.open_report().embedder_device_resolution
        if resolution is None or resolution.effective_device.kind != "cuda":
            raise RuntimeError("forced CUDA did not resolve CUDA")
        engine.write([{"kind": "doc", "body": TEXT, "source_id": "slice130-lifecycle"}])
        engine.drain(timeout_s=60)
        engines.append(engine)
        expected.append(vector_hash(engine.embed(TEXT)))

    for line in sys.stdin:
        operation = json.loads(line)["operation"]
        try:
            if operation == "ready":
                if not args.lifecycle:
                    open_engine()
                result = {"pid": os.getpid(), "package": fathomdb.__file__, "native": _native.__file__}
                if engines:
                    resolution = engines[0].open_report().embedder_device_resolution
                    info = resolution.effective_device.cuda_device
                    result["allocator"] = asdict(info.cuda_allocator) if info.cuda_allocator else None
                    result["selected_cuda_uuid"] = resolution.selected_cuda_uuid
            elif operation in ("open-one", "open-two"):
                for _ in range(2 if operation == "open-two" else 1):
                    open_engine()
                result = {"outputs": expected}
            elif operation == "snapshot":
                result = {"raw_observation": json.loads(_native._cuda_pool_observation_for_qualification())}
            elif operation == "iteration":
                latency = {}
                start = time.monotonic()
                single = vector_hash(engines[0].embed(TEXT))
                latency["embed"] = (time.monotonic() - start) * 1000
                start = time.monotonic()
                batch = fathomdb.embed_batch_cls([f"soak {iteration} item {k} about pools" for k in range(32)])
                latency["batch32"] = (time.monotonic() - start) * 1000
                if len(batch) != 32:
                    raise ValueError("batch32 output count changed")
                batch_hashes = [vector_hash(v) for v in batch]
                start = time.monotonic()
                ranked = fathomdb.rerank("Which device shares DRAM?", PASSAGES, 2)
                latency["rerank"] = (time.monotonic() - start) * 1000
                scores = [r["ce_score"] for r in ranked]
                if len(scores) != 2 or any(v is None or not math.isfinite(v) for v in scores):
                    raise ValueError("reranker output missing/nonfinite")
                if expected_scores is None:
                    expected_scores = scores
                result = {"latency_ms": latency, "mismatches": int(single != expected[0]) + int(scores != expected_scores),
                          "outputs": {"embed_sha256": single, "batch32_sha256": batch_hashes, "rerank": ranked},
                          "input_iteration": iteration}
                iteration += 1
            elif operation == "survivor":
                result = {"unchanged": vector_hash(engines[-1].embed(TEXT)) == expected[-1]}
            elif operation == "singleton-cls":
                result = {"output_sha256": vector_hash(fathomdb.embed_batch_cls([TEXT])[0])}
            elif operation == "singleton-reranker":
                result = {"outputs": fathomdb.rerank("Which device shares DRAM?", PASSAGES, 2)}
            elif operation == "close-first":
                engines[0].close()
                result = {"closed_monotonic_s": time.monotonic()}
            elif operation == "close-last":
                engines[-1].close()
                result = {"closed_monotonic_s": time.monotonic()}
            elif operation == "drop-all":
                engines.clear()
                result = {"dropped_monotonic_s": time.monotonic()}
            elif operation == "close":
                for engine in engines:
                    engine.close()
                result = {"closed_monotonic_s": time.monotonic()}
            elif operation == "exit":
                result = {"unclosed_owners": len(engines)}
            else:
                raise ValueError(f"unknown operation {operation}")
            print(json.dumps({"operation": operation, "ok": True, "result": result}, allow_nan=False), flush=True)
            if operation in ("close", "exit"):
                return 0
        except Exception as error:
            print(json.dumps({"operation": operation, "ok": False, "error": f"{type(error).__name__}: {error}"}), flush=True)
            return 1
    raise RuntimeError("supervisor EOF before close/exit")


if __name__ == "__main__":
    raise SystemExit(main())
