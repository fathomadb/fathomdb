#!/usr/bin/env python3
"""Capture a forced CUDA reranker refusal from an installed Python wheel."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

from fathomdb import Engine
from fathomdb.errors import RerankerDevicePolicyError


ARGV = ["/opt/python/cp311-cp311/bin/python", "/fathomdb-harness/forced-reranker-python.py"]


def main() -> int:
    try:
        with tempfile.TemporaryDirectory() as directory:
            Engine.open(str(Path(directory) / "forced-reranker-cuda.fdb"), use_default_embedder=False)
    except RerankerDevicePolicyError as error:
        message = str(error)
        payload = {
            "schema_version": "fathomdb.cuda-forced-reranker-capture/v1",
            "consumer": "python",
            "argv": ARGV,
            "requested_policy": "cuda:0",
            "status": "cuda_unavailable",
            "effective_device": None,
            "reason": "no_visible_cuda_device",
            "error": {
                "type": type(error).__name__,
                "kind": error.kind,
                "ordinal": error.ordinal,
                "message": message,
            },
        }
        sys.stdout.write(json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
        sys.stderr.write(message + "\n")
        return 1
    raise RuntimeError("forced reranker cuda:0 unexpectedly opened successfully")


if __name__ == "__main__":
    raise SystemExit(main())
