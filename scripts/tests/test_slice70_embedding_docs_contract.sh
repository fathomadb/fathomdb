#!/usr/bin/env bash
# Static regression guard for the Slice 70 *current* embedding-device contract.
#
# Historical design records may preserve their original proposal wording, but
# must say explicitly that they are superseded.  The public docs, examples,
# source-adjacent package help, and eval CLI help must describe the one runtime
# policy: unset is auto, cpu is explicit CPU, and CUDA requires cuda:N.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

require() {
  local file="$1" needle="$2"
  if ! grep -Fq "$needle" "$file"; then
    printf 'missing required Slice 70 device-contract text in %s: %s\n' "$file" "$needle" >&2
    exit 1
  fi
}

reject() {
  local file="$1" needle="$2"
  if grep -Fq "$needle" "$file"; then
    printf 'stale Slice 70 device-contract text in %s: %s\n' "$file" "$needle" >&2
    exit 1
  fi
}

require docs/embedder.md 'unset/default policy is `auto` only on a CUDA-capable artifact'
require docs/reference/python-api.md 'Unset is `auto` only on a CUDA-capable artifact'
require src/rust/crates/fathomdb-py/Cargo.toml 'FATHOMDB_EMBED_DEVICE=cuda:N'
require src/python/eval/p0a_batch_e2e.py 'FATHOMDB_EMBED_DEVICE=cuda:0'
require src/rust/crates/fathomdb-embedder/examples/gpu_speedup.rs 'auto (unset; CPU-only artifacts report cuda_not_compiled'
require src/rust/crates/fathomdb-py/src/lib.rs 'unset means `auto`'

reject src/rust/crates/fathomdb-py/Cargo.toml 'FATHOMDB_EMBED_DEVICE=cuda.'
reject src/python/eval/p0a_batch_e2e.py 'FATHOMDB_EMBED_DEVICE=cuda or'
reject src/rust/crates/fathomdb-py/src/lib.rs 'CPU default; `cuda:N`'

require dev/adr/ADR-0.8.16-onnx-embedder-backend.md 'Historical record — Slice 70 supersedes this ADR for current runtime device policy.'
require dev/design/gpu-device-allocation-policy.md 'Historical design snapshot — not a current runtime contract.'

printf 'PASS  Slice 70 embedding documentation contract\n'
