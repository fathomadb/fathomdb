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

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'cuda-preflight: required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

for command in nvidia-smi nvcc docker readelf ldd maturin python3 unzip npm; do
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
  printf 'driverless_image=%s\n' "$CUDA_DRIVERLESS_IMAGE"
  printf '\n[nvidia-smi]\n'; nvidia-smi --query-gpu=index,name,driver_version --format=csv,noheader
  printf '\n[nvcc]\n'; "$CUDA_TOOLKIT_ROOT/bin/nvcc" --version
  printf '\n[docker]\n'; docker version --format '{{.Server.Version}}'
  printf '\n[maturin]\n'; maturin --version
} | tee "$WITNESS_DIR/environment.txt"

printf 'cuda-preflight: build Linux CUDA N-API artifact\n'
"$SCRIPT_DIR/build-napi-cuda.sh"
NAPI_BINARY="$(find "$REPO_ROOT/src/ts" -maxdepth 1 -type f -name 'fathomdb.linux-x64-gnu.node' -print -quit)"
if [ -z "$NAPI_BINARY" ]; then
  printf 'cuda-preflight: Linux CUDA N-API build produced no linux-x64-gnu artifact\n' >&2
  exit 1
fi

printf 'cuda-preflight: build Linux CUDA Python wheel\n'
(
  cd "$REPO_ROOT/src/python"
  "$CUDA_TOOLKIT_ROOT/bin/nvcc" --version >/dev/null
  maturin build --release --out "$WITNESS_DIR/python-dist" \
    --features "$CUDA_PYTHON_FEATURES" \
    --manylinux "$CUDA_MANYLINUX" \
    --interpreter python3.11
)
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

printf 'cuda-preflight: prove CPU default in a driverless container\n'
docker run --rm --network none \
  --mount "type=bind,src=$WHEEL,dst=/input/fathomdb.whl,readonly" \
  "$CUDA_DRIVERLESS_IMAGE" \
  sh -ceu '
    test ! -e /dev/nvidiactl
    python -m pip install --no-deps /input/fathomdb.whl
    FATHOMDB_EMBED_DEVICE=cpu python - <<"PY"
import pathlib
import sys
import tempfile

from fathomdb import Engine

with tempfile.TemporaryDirectory() as directory:
    db_path = pathlib.Path(directory) / "driverless.fdb"
    engine = Engine.open(str(db_path))
    engine.write([{"kind": "doc", "body": "{}", "source_id": "smoke:cuda-driverless"}])
    engine.search("smoke")
    engine.close()
print("driverless CPU smoke: ok")
PY
  ' | tee "$WITNESS_DIR/driverless-cpu-smoke.txt"

printf 'cuda-preflight: pass; witness at %s\n' "$WITNESS_DIR"
