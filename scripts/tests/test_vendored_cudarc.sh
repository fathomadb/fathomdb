#!/usr/bin/env bash
# Runs the FathomDB-owned unit tests inside the vendored cudarc 0.19.7
# (third_party/cudarc-0.19.7, modules `fathomdb_alloc_fallback` and
# `mem_pool`): allocator selection, zero-length synchronous allocation and the
# explicit memory pool primitive. The vendor copy is not a
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
  -- --test-threads=1 --nocapture fathomdb_alloc_fallback safe::mem_pool
