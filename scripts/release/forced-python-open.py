#!/usr/bin/env python3
"""Exercise forced CUDA selection through an installed Python candidate."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

from fathomdb import Engine
from fathomdb.errors import EmbedDevicePolicyError


ARGV = ["python", "/fathomdb-harness/forced-python-open.py"]


def main() -> int:
    try:
        with tempfile.TemporaryDirectory() as directory:
            Engine.open(str(Path(directory) / "forced-cuda.fdb"), use_default_embedder=True)
    except EmbedDevicePolicyError as error:
        message = str(error)
        payload = {
            "schema_version": "fathomdb.cuda-forced-device-capture/v1",
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
    raise RuntimeError("forced cuda:0 unexpectedly opened successfully")


if __name__ == "__main__":
    raise SystemExit(main())
