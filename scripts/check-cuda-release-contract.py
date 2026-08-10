#!/usr/bin/env python3
"""Validate the 0.8.23 Linux CUDA release-contract seam without building it.

The real CUDA build and smokes run only from the workflow-restricted release
runner group.  This checker makes the inputs to that run reviewable in ordinary
CPU CI: the feature forwarding, the Linux-only build entry points, and the
non-publishing preflight job must agree exactly.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 CI fallback
    import tomli as tomllib  # type: ignore[import-not-found]


ROOT = Path(os.environ.get("REPO_ROOT", Path(__file__).resolve().parents[1]))
NAPI_MANIFEST = ROOT / "src/rust/crates/fathomdb-napi/Cargo.toml"
TS_PACKAGE = ROOT / "src/ts/package.json"
WORKFLOW = ROOT / ".github/workflows/release.yml"
CUDA_CONTRACT = ROOT / "scripts/release/cuda-artifact-contract.sh"
CUDA_NAPI_BUILD = ROOT / "scripts/release/build-napi-cuda.sh"
CUDA_PREFLIGHT = ROOT / "scripts/release/cuda-preflight.sh"

NAPI_CUDA_FEATURE = ["default-embedder", "fathomdb-engine/embed-cuda"]
NAPI_CUDA_BUILD = "bash ../../scripts/release/build-napi-cuda.sh"
PYTHON_CUDA_FEATURES = "pyo3/extension-module,embed-cuda"
RUNNER_LABELS = ("self-hosted", "Linux", "X64", "gpu", "cuda-12")


def fail(message: str) -> None:
    print(f"cuda-release-contract: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_toml(path: Path) -> dict[str, object]:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot read {path}: {error}")


def load_json(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{path} must contain a JSON object")
    return value


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        fail(f"cannot read {path}: {error}")


def workflow_job(name: str) -> str:
    try:
        text = WORKFLOW.read_text(encoding="utf-8")
    except OSError as error:
        fail(f"cannot read {WORKFLOW}: {error}")
    match = re.search(
        rf"^  {re.escape(name)}:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
        text,
        re.MULTILINE | re.DOTALL,
    )
    if match is None:
        fail(f"release workflow lacks {name!r} job")
    return match.group(0)


def require_fragment(block: str, fragment: str, label: str) -> None:
    if fragment not in block:
        fail(f"{label} is missing {fragment!r}")


def main() -> None:
    manifest = load_toml(NAPI_MANIFEST)
    features = manifest.get("features")
    if not isinstance(features, dict):
        fail("fathomdb-napi Cargo.toml has no [features] table")
    if features.get("embed-cuda") != NAPI_CUDA_FEATURE:
        fail(
            "fathomdb-napi embed-cuda feature must forward exactly "
            f"{NAPI_CUDA_FEATURE!r}; got {features.get('embed-cuda')!r}"
        )

    package = load_json(TS_PACKAGE)
    scripts = package.get("scripts")
    if not isinstance(scripts, dict):
        fail("src/ts/package.json has no scripts object")
    if scripts.get("build:native:cuda") != NAPI_CUDA_BUILD:
        fail(
            "src/ts/package.json build:native:cuda must own the Linux CUDA N-API "
            f"arguments exactly; got {scripts.get('build:native:cuda')!r}"
        )

    contract = read_text(CUDA_CONTRACT)
    require_fragment(
        contract,
        "CUDA_NAPI_FEATURES='embed-cuda'",
        "CUDA artifact contract",
    )
    require_fragment(
        contract,
        "CUDA_PYTHON_FEATURES='pyo3/extension-module,embed-cuda'",
        "CUDA artifact contract",
    )
    require_fragment(contract, "CUDA_MANYLINUX='2_28'", "CUDA artifact contract")
    require_fragment(contract, "CUDA_COMPUTE_CAP='75'", "CUDA artifact contract")
    require_fragment(
        contract,
        "CUDA_TOOLKIT_ROOT='/usr/local/cuda-12.6'",
        "CUDA artifact contract",
    )
    for fragment in (
        "CUDA_MANYLINUX_IMAGE=",
        "CUDA_MANYLINUX_PYTHON=",
        "CUDA_DRIVERLESS_PYTHON_IMAGE=",
        "CUDA_DRIVERLESS_NODE_IMAGE=",
        "CUDA_DEFAULT_EMBEDDER_HF_REPO=",
        "CUDA_DEFAULT_EMBEDDER_HF_REVISION=",
    ):
        require_fragment(contract, fragment, "CUDA artifact contract")
    napi_build = read_text(CUDA_NAPI_BUILD)
    require_fragment(napi_build, '"$CUDA_NAPI_FEATURES"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export CUDA_PATH="$CUDA_TOOLKIT_ROOT"', "CUDA N-API build wrapper")
    require_fragment(napi_build, "export CUDA_COMPUTE_CAP", "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export PATH="$CUDA_TOOLKIT_ROOT/bin:$PATH"', "CUDA N-API build wrapper")
    preflight = read_text(CUDA_PREFLIGHT)
    for fragment in (
        '"$CUDA_TOOLKIT_ROOT/bin/nvcc" --version',
        'maturin build --release --out /witness/python-dist',
        '--features "$CUDA_PYTHON_FEATURES"',
        '--manylinux "$CUDA_MANYLINUX"',
        'readelf -d "$NAPI_BINARY"',
        'readelf -d "$PYTHON_EXTENSION"',
        'docker run --rm --network none',
        'CUDA_MANYLINUX_IMAGE',
        'docker image inspect "$CUDA_MANYLINUX_IMAGE"',
        'maturin --version',
        'rustc --version',
        'CUDACXX=/opt/cuda/bin/nvcc',
        '--mount "type=bind,src=$CUDA_TOOLKIT_ROOT,dst=/opt/cuda,readonly"',
        '--mount "type=bind,src=$REPO_ROOT,dst=/workspace"',
        'manylinux-build.txt',
        'CUDA_DRIVERLESS_PYTHON_IMAGE',
        'CUDA_DRIVERLESS_NODE_IMAGE',
        'DEFAULT_EMBEDDER_HF_HOME',
        'Engine.open(str(db_path), use_default_embedder=True)',
        'engine.embed("driverless Python CUDA-capable default-embedder proof")',
        'useDefaultEmbedder: true',
        'await engine.embed("driverless N-API CUDA-capable default-embedder proof")',
        'npm install --offline --ignore-scripts --no-audit --no-fund',
        'test ! -e /dev/nvidiactl',
    ):
        require_fragment(preflight, fragment, "CUDA preflight")
    if "Engine.open(str(db_path), use_default_embedder=False)" in preflight:
        fail("CUDA preflight must not use a no-embedder Python smoke")
    if preflight.count("--network none") < 2:
        fail("CUDA preflight must isolate both installed-artifact CPU smokes from the network")
    if preflight.count(
        '--mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly"'
    ) < 2:
        fail("CUDA preflight must mount the pinned local default-embedder mirror into both smokes")
    if preflight.count("-e HF_HOME=/fathomdb-hf") < 2:
        fail("CUDA preflight must make both smokes load only from the mounted local mirror")

    job = workflow_job("cuda-contract-preflight")
    require_fragment(
        job,
        "if: ${{ github.event_name == 'workflow_dispatch' && inputs.dry_run == true }}",
        "cuda-contract-preflight",
    )
    require_fragment(job, "runs-on: [self-hosted, Linux, X64, gpu, cuda-12]", "cuda-contract-preflight")
    for label in RUNNER_LABELS:
        require_fragment(job, label, "cuda-contract-preflight runner labels")
    require_fragment(job, "needs: verify-release", "cuda-contract-preflight")
    if not re.search(
        r"^    permissions:\n      contents: read\n",
        job,
        re.MULTILINE,
    ):
        fail("cuda-contract-preflight must declare job-level read-only contents permission")
    if "contents: write" in job or "id-token: write" in job:
        fail("cuda-contract-preflight must not inherit release publishing permissions")
    require_fragment(job, "bash scripts/release/cuda-preflight.sh", "cuda-contract-preflight")
    require_fragment(job, "${{ env.RELEASE_CHECKOUT_REF }}", "cuda-contract-preflight checkout")
    require_fragment(job, "name: cuda-preflight-witness", "cuda-contract-preflight witness upload")
    require_fragment(job, "${{ github.workspace }}/cuda-preflight-witness", "cuda-contract-preflight witness path")

    python_build = workflow_job("build-python")
    require_fragment(python_build, "args: --release --out dist --features pyo3/extension-module,default-embedder", "ordinary build-python")
    if PYTHON_CUDA_FEATURES in python_build:
        fail("ordinary cross-platform build-python must stay CPU-only; CUDA belongs to the restricted preflight")

    ordinary_napi_build = workflow_job("build-napi")
    require_fragment(ordinary_napi_build, "run: npm run build:native", "ordinary build-napi")
    if "build:native:cuda" in ordinary_napi_build:
        fail("ordinary cross-platform build-napi must stay CPU-only; CUDA belongs to the restricted preflight")

    print("cuda-release-contract: pass")


if __name__ == "__main__":
    main()
