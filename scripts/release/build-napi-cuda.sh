#!/usr/bin/env bash
# Build the Linux CUDA N-API artifact from the release contract.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
REQUESTED_CUDA_NAPI_FEATURES="${CUDA_NAPI_FEATURES:-}"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"
if [ -n "$REQUESTED_CUDA_NAPI_FEATURES" ]; then
  CUDA_NAPI_FEATURES="$REQUESTED_CUDA_NAPI_FEATURES"
fi

if [ ! -x "$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" ]; then
  printf 'build-napi-cuda: expected host CUDA compiler at %s/bin/nvcc\n' "$CUDA_NAPI_HOST_TOOLKIT_ROOT" >&2
  exit 1
fi
if ! "$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" --version | grep -F "$CUDA_NAPI_HOST_NVCC_VERSION"; then
  printf 'build-napi-cuda: host CUDA compiler does not match the release contract\n' >&2
  exit 1
fi
for compiler in "$CUDA_NAPI_HOST_CC" "$CUDA_NAPI_HOST_CXX"; do
  if [ ! -x "$compiler" ]; then
    printf 'build-napi-cuda: expected GCC 13 compiler at %s\n' "$compiler" >&2
    exit 1
  fi
done
if ! "$CUDA_NAPI_HOST_CC" --version | grep -F "$CUDA_NAPI_HOST_GCC_VERSION"; then
  printf 'build-napi-cuda: host C compiler does not match the release contract\n' >&2
  exit 1
fi
if ! "$CUDA_NAPI_HOST_CXX" --version | grep -F "$CUDA_NAPI_HOST_GCC_VERSION"; then
  printf 'build-napi-cuda: host C++ compiler does not match the release contract\n' >&2
  exit 1
fi
export CUDA_PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT"
export CUDACXX="$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc"
export CC="$CUDA_NAPI_HOST_CC"
export CXX="$CUDA_NAPI_HOST_CXX"
export CUDAHOSTCXX="$CUDA_NAPI_HOST_CXX"
export NVCC_CCBIN="$CUDA_NAPI_HOST_CXX"
export CUDA_COMPUTE_CAP
# The CUDA runtime stays a normal toolkit dependency for the generated kernels;
# the Candle source pin keeps the NVIDIA driver dynamically loaded instead.
export LIBRARY_PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64${LIBRARY_PATH:+:$LIBRARY_PATH}"
export PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin:$PATH"

cd "$REPO_ROOT/src/ts"
npm exec -- napi build --platform --release \
  --cargo-cwd ../rust/crates/fathomdb-napi \
  --features "$CUDA_NAPI_FEATURES" \
  --js false
