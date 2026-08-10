#!/usr/bin/env bash
# Build the Linux CUDA N-API artifact from the release contract.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"

if [ ! -x "$CUDA_TOOLKIT_ROOT/bin/nvcc" ]; then
  printf 'build-napi-cuda: expected CUDA compiler at %s/bin/nvcc\n' "$CUDA_TOOLKIT_ROOT" >&2
  exit 1
fi
export CUDA_PATH="$CUDA_TOOLKIT_ROOT"
export CUDACXX="$CUDA_TOOLKIT_ROOT/bin/nvcc"
export CUDA_COMPUTE_CAP
export PATH="$CUDA_TOOLKIT_ROOT/bin:$PATH"

cd "$REPO_ROOT/src/ts"
npm exec -- napi build --platform --release \
  --cargo-cwd ../rust/crates/fathomdb-napi \
  --features "$CUDA_NAPI_FEATURES" \
  --js false
