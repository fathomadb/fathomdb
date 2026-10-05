#!/usr/bin/env bash
# Runs the FathomDB-owned unit tests inside the vendored cudarc 0.19.7
# (third_party/cudarc-0.19.7, module `fathomdb_alloc_fallback`): allocator
# selection and zero-length synchronous allocation. The vendor copy is not a
# workspace member, so it is tested from a scratch copy seeded with the
# workspace lockfile. The pure tests run everywhere; the GPU tests print SKIP
# on hosts without a CUDA driver and device.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cp -a "$REPO_ROOT/third_party/cudarc-0.19.7/." "$WORK/cudarc/"
cp "$REPO_ROOT/Cargo.lock" "$WORK/cudarc/Cargo.lock"

# An explicit CUDA version and runtime driver loading keep the build free of
# nvcc and libcuda, so hosts without a CUDA toolkit can compile the tests.
cargo test \
  --manifest-path "$WORK/cudarc/Cargo.toml" \
  --target-dir "$REPO_ROOT/target/vendored-cudarc" \
  --lib \
  --no-default-features \
  --features std,driver,cuda-12060,dynamic-loading \
  fathomdb_alloc_fallback \
  -- --test-threads=1 --nocapture
