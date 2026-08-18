#!/usr/bin/env python3
"""Execute reranker refusal harnesses without allowing a default embedder load."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PY_HARNESS = ROOT / "scripts/release/forced-reranker-python.py"
NAPI_HARNESS = ROOT / "scripts/release/forced-reranker-napi.mjs"
MESSAGE = "cuda:0 requested for reranking but unavailable: NoVisibleCudaDevice"


def run_python(root: Path) -> None:
    package = root / "python/fathomdb"
    package.mkdir(parents=True)
    (package / "__init__.py").write_text(
        "from .errors import RerankerDevicePolicyError\n"
        "class Engine:\n"
        "    @staticmethod\n"
        "    def open(path, use_default_embedder=False):\n"
        "        import os\n"
        "        open(os.environ['HARNESS_CALL_LOG'], 'w').write(str(use_default_embedder))\n"
        "        raise RerankerDevicePolicyError()\n"
    )
    (package / "errors.py").write_text(
        "class RerankerDevicePolicyError(Exception):\n"
        "    kind = 'no_visible_cuda_device'\n"
        "    ordinal = 0\n"
        "    def __str__(self): return 'cuda:0 requested for reranking but unavailable: NoVisibleCudaDevice'\n"
    )
    log = root / "python-call.txt"
    env = os.environ | {"PYTHONPATH": str(root / "python"), "HARNESS_CALL_LOG": str(log)}
    result = subprocess.run([sys.executable, str(PY_HARNESS)], text=True, capture_output=True, env=env, check=False)
    assert result.returncode == 1, result
    assert log.read_text() == "False", "forced reranker Python harness loaded the default embedder"
    payload = json.loads(result.stdout)
    assert payload["error"]["type"] == "RerankerDevicePolicyError"
    assert result.stderr == f"{MESSAGE}\n"


def run_napi(root: Path) -> None:
    if shutil.which("node") is None:
        return
    package = root / "node/node_modules/fathomdb"
    package.mkdir(parents=True)
    (package / "package.json").write_text('{"type":"module"}\n')
    (package / "index.js").write_text(
        "import fs from 'node:fs';\n"
        "export class RerankerDevicePolicyError extends Error {\n"
        "  constructor(){ super('cuda:0 requested for reranking but unavailable: NoVisibleCudaDevice'); this.kind='no_visible_cuda_device'; this.ordinal=0; }\n"
        "}\n"
        "export class Engine { static async open(_path, options) {\n"
        "  fs.writeFileSync(process.env.HARNESS_CALL_LOG, String(options.useDefaultEmbedder));\n"
        "  throw new RerankerDevicePolicyError();\n"
        "} }\n"
    )
    log = root / "napi-call.txt"
    result = subprocess.run(
        ["node", str(NAPI_HARNESS)], text=True, capture_output=True,
        env=os.environ | {"NODE_PATH": str(root / "node/node_modules"), "HARNESS_CALL_LOG": str(log)},
        check=False,
    )
    assert result.returncode == 1, result
    assert log.read_text() == "false", "forced reranker N-API harness loaded the default embedder"
    payload = json.loads(result.stdout)
    assert payload["error"]["type"] == "RerankerDevicePolicyError"
    assert result.stderr == f"{MESSAGE}\n"


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        run_python(root)
        run_napi(root)
    print("CUDA reranker forced-refusal harness tests passed")


if __name__ == "__main__":
    main()
