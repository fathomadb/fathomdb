#!/usr/bin/env bash
# Collect fail-closed CUDA build/link/CPU-compatibility evidence before release.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  printf 'usage: %s <witness-directory>\n' "$0" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WITNESS_DIR="$1"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"
DEFAULT_EMBEDDER_HF_HOME="${FATHOMDB_CUDA_PREFLIGHT_HF_HOME:-${HF_HOME:-$HOME/.cache/huggingface}}"
DEFAULT_EMBEDDER_SNAPSHOT="$DEFAULT_EMBEDDER_HF_HOME/hub/models--${CUDA_DEFAULT_EMBEDDER_HF_REPO//\//--}/snapshots/$CUDA_DEFAULT_EMBEDDER_HF_REVISION"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'cuda-preflight: required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

for command in nvidia-smi nvcc docker readelf ldd python3 unzip npm; do
  require_command "$command"
done
if [ ! -x "$CUDA_TOOLKIT_ROOT/bin/nvcc" ]; then
  printf 'cuda-preflight: expected CUDA 12.6 compiler at %s/bin/nvcc\n' "$CUDA_TOOLKIT_ROOT" >&2
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  printf 'cuda-preflight: Docker must be available to prove the driverless CPU container\n' >&2
  exit 1
fi
if ! docker image inspect "$CUDA_MANYLINUX_IMAGE" >/dev/null 2>&1; then
  printf 'cuda-preflight: designated runner lacks required CUDA/manylinux image %s\n' \
    "$CUDA_MANYLINUX_IMAGE" >&2
  printf 'cuda-preflight: provision that image with CUDA 12.6, Rust, maturin, and %s before rerunning\n' \
    "$CUDA_MANYLINUX_PYTHON" >&2
  exit 1
fi
for file_name in config.json tokenizer.json model.safetensors; do
  if [ ! -f "$DEFAULT_EMBEDDER_SNAPSHOT/$file_name" ]; then
    printf 'cuda-preflight: required local default-embedder cache file is absent: %s\n' \
      "$DEFAULT_EMBEDDER_SNAPSHOT/$file_name" >&2
    printf 'cuda-preflight: warm the pinned cache/mirror on the designated runner; driverless smokes never use network\n' >&2
    exit 1
  fi
done

if [ -e "$WITNESS_DIR" ]; then
  printf 'cuda-preflight: witness directory must be new: %s\n' "$WITNESS_DIR" >&2
  exit 1
fi
mkdir -p "$WITNESS_DIR/python-dist" "$WITNESS_DIR/python-unpacked"
export CUDA_PATH="$CUDA_TOOLKIT_ROOT"
export CUDACXX="$CUDA_TOOLKIT_ROOT/bin/nvcc"
export CUDA_COMPUTE_CAP
export PATH="$CUDA_TOOLKIT_ROOT/bin:$PATH"

{
  printf 'generated_at_utc='; date --utc --iso-8601=seconds
  printf 'repository_commit='; git -C "$REPO_ROOT" rev-parse HEAD
  printf 'cuda_toolkit_root=%s\n' "$CUDA_TOOLKIT_ROOT"
  printf 'cuda_manylinux=%s\n' "$CUDA_MANYLINUX"
  printf 'cuda_compute_cap=%s\n' "$CUDA_COMPUTE_CAP"
  printf 'cuda_napi_features=%s\n' "$CUDA_NAPI_FEATURES"
  printf 'cuda_python_features=%s\n' "$CUDA_PYTHON_FEATURES"
  printf 'cuda_manylinux_image=%s\n' "$CUDA_MANYLINUX_IMAGE"
  printf 'cuda_manylinux_python=%s\n' "$CUDA_MANYLINUX_PYTHON"
  printf 'driverless_python_image=%s\n' "$CUDA_DRIVERLESS_PYTHON_IMAGE"
  printf 'driverless_node_image=%s\n' "$CUDA_DRIVERLESS_NODE_IMAGE"
  printf 'default_embedder_hf_home=%s\n' "$DEFAULT_EMBEDDER_HF_HOME"
  printf 'default_embedder_snapshot=%s\n' "$DEFAULT_EMBEDDER_SNAPSHOT"
  printf '\n[nvidia-smi]\n'; nvidia-smi --query-gpu=index,name,driver_version --format=csv,noheader
  printf '\n[nvcc]\n'; "$CUDA_TOOLKIT_ROOT/bin/nvcc" --version
  printf '\n[docker]\n'; docker version --format '{{.Server.Version}}'
  printf '\n[cuda-manylinux image]\n'; docker image inspect --format '{{.Id}} {{.RepoDigests}}' "$CUDA_MANYLINUX_IMAGE"
} | tee "$WITNESS_DIR/environment.txt"

printf 'cuda-preflight: build Linux CUDA N-API artifact\n'
"$SCRIPT_DIR/build-napi-cuda.sh"
NAPI_BINARY="$(find "$REPO_ROOT/src/ts" -maxdepth 1 -type f -name 'fathomdb.linux-x64-gnu.node' -print -quit)"
if [ -z "$NAPI_BINARY" ]; then
  printf 'cuda-preflight: Linux CUDA N-API build produced no linux-x64-gnu artifact\n' >&2
  exit 1
fi

printf 'cuda-preflight: build Linux CUDA Python wheel\n'
docker run --rm \
  --mount "type=bind,src=$REPO_ROOT,dst=/workspace" \
  --mount "type=bind,src=$WITNESS_DIR,dst=/witness" \
  --mount "type=bind,src=$CUDA_TOOLKIT_ROOT,dst=/opt/cuda,readonly" \
  --workdir /workspace/src/python \
  -e CUDA_PATH=/opt/cuda \
  -e CUDACXX=/opt/cuda/bin/nvcc \
  -e CUDA_COMPUTE_CAP \
  -e CUDA_PYTHON_FEATURES \
  -e CUDA_MANYLINUX \
  -e CUDA_MANYLINUX_PYTHON \
  "$CUDA_MANYLINUX_IMAGE" \
  sh -ceu '
    command -v maturin
    maturin --version
    rustc --version
    /opt/cuda/bin/nvcc --version
    maturin build --release --out /witness/python-dist \
      --features "$CUDA_PYTHON_FEATURES" \
      --manylinux "$CUDA_MANYLINUX" \
      --interpreter "$CUDA_MANYLINUX_PYTHON"
  ' | tee "$WITNESS_DIR/manylinux-build.txt"
WHEEL="$(find "$WITNESS_DIR/python-dist" -maxdepth 1 -type f -name '*.whl' -print -quit)"
if [ -z "$WHEEL" ]; then
  printf 'cuda-preflight: CUDA Python build produced no wheel\n' >&2
  exit 1
fi
unzip -q "$WHEEL" -d "$WITNESS_DIR/python-unpacked"
PYTHON_EXTENSION="$(find "$WITNESS_DIR/python-unpacked" -type f -name '*.so' -print -quit)"
if [ -z "$PYTHON_EXTENSION" ]; then
  printf 'cuda-preflight: CUDA wheel contains no Python extension\n' >&2
  exit 1
fi

{
  printf '[node readelf]\n'; readelf -d "$NAPI_BINARY"
  printf '\n[node ldd]\n'; ldd "$NAPI_BINARY" || true
  printf '\n[python readelf]\n'; readelf -d "$PYTHON_EXTENSION"
  printf '\n[python ldd]\n'; ldd "$PYTHON_EXTENSION" || true
} | tee "$WITNESS_DIR/dynamic-dependencies.txt"

printf 'cuda-preflight: stage installed N-API package for the driverless smoke\n'
(
  cd "$REPO_ROOT/src/ts"
  npm exec -- tsc -p tsconfig.build.json
)
NPM_STAGING="$WITNESS_DIR/npm-artifacts"
NPM_MAIN="$NPM_STAGING/main"
NPM_PLATFORM_ROOT="$NPM_STAGING/platforms"
NPM_PLATFORM="$NPM_PLATFORM_ROOT/linux-x64-gnu"
mkdir -p "$NPM_MAIN" "$NPM_PLATFORM"
cp "$REPO_ROOT/src/ts/package.json" "$REPO_ROOT/src/ts/LICENSE" "$NPM_MAIN/"
cp -R "$REPO_ROOT/src/ts/dist" "$NPM_MAIN/dist"
cp "$REPO_ROOT/src/ts/npm/linux-x64-gnu/package.json" "$REPO_ROOT/src/ts/npm/linux-x64-gnu/LICENSE" \
  "$NPM_PLATFORM/"
cp "$NAPI_BINARY" "$NPM_PLATFORM/fathomdb.linux-x64-gnu.node"
bash "$SCRIPT_DIR/npm-inject-optional-deps.sh" "$NPM_MAIN" "$NPM_PLATFORM_ROOT"
NPM_PLATFORM_TARBALL="$(cd "$NPM_PLATFORM" && npm pack --silent)"
NPM_MAIN_TARBALL="$(cd "$NPM_MAIN" && npm pack --silent)"

printf 'cuda-preflight: prove the installed Python wheel defaults to CPU in a driverless container\n'
docker run --rm --network none \
  --mount "type=bind,src=$WHEEL,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  -e HF_HOME=/fathomdb-hf \
  -e FATHOMDB_EMBED_DEVICE=cpu \
  "$CUDA_DRIVERLESS_PYTHON_IMAGE" \
  sh -ceu '
    test ! -e /dev/nvidiactl
    python -m pip install --no-deps /input/fathomdb.whl
    python - <<"PY"
import pathlib
import sys
import tempfile

from fathomdb import Engine

with tempfile.TemporaryDirectory() as directory:
    db_path = pathlib.Path(directory) / "driverless.fdb"
    engine = Engine.open(str(db_path), use_default_embedder=True)
    assert len(engine.embed("driverless Python CUDA-capable default-embedder proof")) == 384
    engine.write([{"kind": "doc", "body": "{}", "source_id": "smoke:cuda-driverless"}])
    engine.search("smoke")
    engine.close()
print("driverless installed Python CUDA-capable default-embedder CPU smoke: ok")
PY
  ' | tee "$WITNESS_DIR/driverless-python-cpu-smoke.txt"

printf 'cuda-preflight: prove the installed N-API package defaults to CPU in a driverless container\n'
docker run --rm --network none \
  --mount "type=bind,src=$NPM_MAIN/$NPM_MAIN_TARBALL,dst=/input/fathomdb.tgz,readonly" \
  --mount "type=bind,src=$NPM_PLATFORM/$NPM_PLATFORM_TARBALL,dst=/input/fathomdb-linux-x64-gnu.tgz,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  -e HF_HOME=/fathomdb-hf \
  -e FATHOMDB_EMBED_DEVICE=cpu \
  "$CUDA_DRIVERLESS_NODE_IMAGE" \
  sh -ceu '
    test ! -e /dev/nvidiactl
    mkdir /consumer
    cd /consumer
    cat > package.json <<"JSON"
{
  "private": true,
  "type": "module",
  "dependencies": {
    "fathomdb": "file:/input/fathomdb.tgz",
    "fathomdb-linux-x64-gnu": "file:/input/fathomdb-linux-x64-gnu.tgz"
  }
}
JSON
    npm install --offline --ignore-scripts --no-audit --no-fund
    node --input-type=module - <<"JS"
import { Engine } from "fathomdb";

const engine = await Engine.open("/tmp/driverless-node.fdb", { useDefaultEmbedder: true });
const vector = await engine.embed("driverless N-API CUDA-capable default-embedder proof");
if (vector.length !== 384) throw new Error("expected 384-vector, got " + vector.length);
await engine.write([{ kind: "doc", body: "{}", sourceId: "smoke:cuda-driverless-napi" }]);
await engine.search("smoke");
await engine.close();
console.log("driverless installed N-API CUDA-capable default-embedder CPU smoke: ok");
JS
  ' | tee "$WITNESS_DIR/driverless-napi-cpu-smoke.txt"

printf 'cuda-preflight: pass; witness at %s\n' "$WITNESS_DIR"
