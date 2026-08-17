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
require docs/reference/typescript-api.md 'Unset is `auto` only on a CUDA-capable artifact'
require docs/reference/typescript-api.md 'End users of a CUDA-capable package do not rebuild to switch between CPU and GPU.'
require dev/design/0.8.23-slice-10-cuda-contract.md 'On a CUDA-capable artifact the unset policy is `auto`, and CPU is an explicit selection.'
require dev/design/0.8.23-slice-10-cuda-contract.md 'device-selection variables absent (`auto` resolves to typed CPU)'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'Driverless unset/`auto` smokes use network-isolated containers'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'The `env -i` allowlist leaves `FATHOMDB_EMBED_DEVICE` absent'
require dev/design/0.8.23-gpu-artifacts.md 'with the device setting absent (`auto` resolves to typed CPU).'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'driverless CPU with the policy unset/`auto`'
require dev/interfaces/rust.md 'A forced-CUDA failure remains `EngineOpenError::EmbedDevicePolicy`'
require dev/interfaces/python.md 'Forced CUDA is an open error, never a CPU report.'
require dev/interfaces/typescript.md 'Forced CUDA is an open rejection, never a CPU report.'
require src/rust/crates/fathomdb-py/Cargo.toml 'FATHOMDB_EMBED_DEVICE=cuda:N'
require src/python/eval/p0a_batch_e2e.py 'FATHOMDB_EMBED_DEVICE=cuda:0'
require src/rust/crates/fathomdb-embedder/examples/gpu_speedup.rs 'auto (unset; CPU-only artifacts report cuda_not_compiled'
require src/rust/crates/fathomdb-py/src/lib.rs 'unset means `auto`'

reject src/rust/crates/fathomdb-py/Cargo.toml 'FATHOMDB_EMBED_DEVICE=cuda.'
reject src/python/eval/p0a_batch_e2e.py 'FATHOMDB_EMBED_DEVICE=cuda or'
reject src/rust/crates/fathomdb-py/src/lib.rs 'CPU default; `cuda:N`'
reject docs/reference/typescript-api.md 'build-time `embed-cuda` feature plus'
reject docs/embedder.md 'fails open if it cannot be used'

require dev/adr/ADR-0.8.16-onnx-embedder-backend.md 'Historical record — Slice 70 supersedes this ADR for current runtime device policy.'
require dev/design/gpu-device-allocation-policy.md 'Historical design snapshot — not a current runtime contract.'
require dev/design/0.8.1-embedder-gpu-and-portability.md 'Historical design record — Slice 70 supersedes this document for current runtime device policy.'
require dev/design/gpu-eval-activities-policy.md 'Historical runtime-policy wording below is superseded by Slice 70.'
require dev/design/0.8.16-slice-0-f9-onnx-design.md 'Historical design record — Slice 70 supersedes this document for current runtime device policy.'
require dev/design/0.8.23-embedding-configuration-feedback.md 'Slice 70 owns current runtime device policy.'
require dev/design/0.8.23-gpu-artifacts.md 'Slice 70 owns runtime device policy; this document retains only the Slice 10/20 artifact-evidence plan.'
reject dev/design/0.8.23-gpu-artifacts.md 'CPU would remain the default'
reject dev/design/0.8.23-gpu-artifacts.md 'existing loud CPU fallback behavior'
reject dev/design/0.8.23-slice-10-cuda-contract.md 'CPU remains the normal public-library default.'
reject dev/design/0.8.23-slice-10-cuda-contract.md 'CPU-default'
reject dev/design/0.8.23-slice-10-cuda-contract.md 'FATHOMDB_EMBED_DEVICE=cpu'
reject dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'CPU-default'
reject dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'FATHOMDB_EMBED_DEVICE=cpu'

printf 'PASS  Slice 70 embedding documentation contract\n'
