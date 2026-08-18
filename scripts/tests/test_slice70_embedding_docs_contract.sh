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

# These rows are the public doctor-gpu contract. Keep both documents' matrix
# rows byte-for-byte aligned so a prose summary cannot soften one outcome.
doctor_matrix_rows=(
  '| `cpu` (any artifact) | none | `selected_cpu_no_cuda` | `cpu` | `[]` | `null` | `0` |'
  '| `auto`, CPU-only artifact | none | `cuda_not_compiled` | `cpu` | `[]` | `null` | `0` |'
  '| `auto`, unavailable evidence | driver-presence, enumeration, or ordinal mapping | `cuda_unavailable` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `0` |'
  '| `auto`, listed compatibility/architecture evidence | enumeration or mapped-device probe | `cuda_incompatible` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `0` |'
  '| `auto`, unknown, OOM, or allocation/provider failure | enumeration or mapped-device probe | `probe_failed` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `70` |'
  '| `auto`, selected device allocation/provider probe succeeds | enumeration + mapped-device probe | `selected_cuda` | selected `cuda:N` | observed inventory | matching UUID | `0` |'
  '| forced `cuda:N`, CUDA not compiled | none | `cuda_not_compiled` | `null` | `[]` | `null` | `65` |'
  '| forced `cuda:N`, unavailable evidence | driver-presence, enumeration, or ordinal mapping | `cuda_unavailable` | `null` | `[]` before inventory; otherwise observed inventory | `null` | `65` |'
  '| forced `cuda:N`, listed compatibility/architecture evidence | enumeration or mapped-device probe | `cuda_incompatible` | `null` | `[]` before inventory; otherwise observed inventory | `null` | `65` |'
  '| forced `cuda:N`, unknown, OOM, or allocation/provider failure | enumeration or mapped-device probe | `probe_failed` | `null` | `[]` before inventory; otherwise observed inventory | `null` | `70` |'
  '| forced `cuda:N`, selected device allocation/provider probe succeeds | enumeration + mapped-device probe | `selected_cuda` | selected `cuda:N` | observed inventory | matching UUID | `0` |'
  '| malformed, legacy, or otherwise invalid policy | none | `invalid_policy` | `null` | `[]` | `null` | `70` |'
)

require_doctor_matrix() {
  local file="$1" row index=0 matrix_output
  local -a actual_rows=()
  matrix_output="$(
    awk '
      /^\| Requested policy \/ observation / { in_matrix = 1; next }
      in_matrix && /^\| --- / { next }
      in_matrix && /^\|/ { print; next }
      in_matrix { exit }
    ' "$file"
  )"
  if [[ -n "$matrix_output" ]]; then
    mapfile -t actual_rows <<<"$matrix_output"
  fi
  if [[ "${#actual_rows[@]}" -ne "${#doctor_matrix_rows[@]}" ]]; then
    printf 'Slice 70 doctor matrix in %s has %d rows; expected exactly %d\n' \
      "$file" "${#actual_rows[@]}" "${#doctor_matrix_rows[@]}" >&2
    exit 1
  fi
  for row in "${doctor_matrix_rows[@]}"; do
    if [[ "${actual_rows[$index]}" != "$row" ]]; then
      printf 'Slice 70 doctor matrix row %d differs in %s\nexpected: %s\nactual:   %s\n' \
        "$((index + 1))" "$file" "$row" "${actual_rows[$index]}" >&2
      exit 1
    fi
    index=$((index + 1))
  done
}

require docs/embedder.md 'unset/default policy is `auto` only on a CUDA-capable artifact'
require docs/reference/python-api.md 'Unset is `auto` only on a CUDA-capable artifact'
require docs/reference/typescript-api.md 'Unset is `auto` only on a CUDA-capable artifact'
require docs/reference/typescript-api.md 'End users of a CUDA-capable package do not rebuild to switch between CPU and GPU.'
require dev/design/0.8.23-slice-10-cuda-contract.md 'On a CUDA-capable artifact the unset policy is `auto`, and CPU is an explicit selection.'
require dev/design/0.8.23-slice-10-cuda-contract.md 'device-selection variables absent (`auto` resolves to typed CPU)'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'Driverless unset/`auto` smokes use network-isolated containers'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'The `env -i` allowlist leaves `FATHOMDB_EMBED_DEVICE` absent'
require dev/design/0.8.23-slice-10-cuda-contract.md 'forced `cuda:N` unavailable/incompatible smoke'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'versioned Linux x86_64 `fathomdb` CLI archive'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'CLI archive SHA-256 digest'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'CLI driverless unset/`auto` CPU smoke'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'CLI compatible-GPU `auto` smoke'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'CLI forced `cuda:N` unavailable/incompatible smoke'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'strict manifest/verifier topology'
require dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'candidate-only and no-publication boundary'
require dev/design/0.8.23-gpu-artifacts.md 'with the device setting absent (`auto` resolves to typed CPU).'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'driverless CPU with the policy unset/`auto`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'requires forced-`cuda:N` evidence in both the Slice 10 and Slice 20 designs.'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'Doctor exit matrix'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`probe_failed`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'typed CPU `effective_device`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'Thus forced `cuda:N` reports no'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`effective_device` for not-compiled/unavailable/incompatible outcomes'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'does not create `init`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'no SDK device setter'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`visible_ordinal`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`selected_uuid`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'schema_version: "fathomdb.doctor.gpu.v1"'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`DoctorGpuDiagnosticResult`, distinct from `DeviceResolution`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`CudaProbeError::ProbeFailed` maps to `probe_failed`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`CUDA_ERROR_SYSTEM_DRIVER_MISMATCH`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`CUBLAS_STATUS_ARCH_MISMATCH`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`CUDA_ERROR_OUT_OF_MEMORY`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`CUBLAS_STATUS_ALLOC_FAILED`'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'The semantic matrix has exactly twelve rows.'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'AC70-D3 runs eighteen deterministic process cases'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'Build target, CUDA toolkit, and driver-version provenance remain Slice 10/20 artifact-witness facts'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'ONNX uses its dedicated strict provider resolver'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'non-shipping subprocess fixture dispatcher'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'No product binary argument or environment variable selects a fixture.'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`strace -f` records file and network syscalls'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md '`reason` and `selected_uuid` are always present'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'The normative text output is exactly'
require dev/interfaces/cli.md '`reason` and `selected_uuid` are always present'
require dev/interfaces/cli.md 'The normative text output is exactly'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'minimal CUDA'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'allocation/provider probe.'
require dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'never performs a model load, download, or'
require dev/design/0.8.23-slice-70-tdd-plan.md 'auto unavailable evidence (missing driver library,'
require dev/design/0.8.23-slice-70-tdd-plan.md 'unknown/OOM/allocation evidence -> `probe_failed`'
require dev/design/0.8.23-slice-70-tdd-plan.md 'not-compiled/unavailable/incompatible -> no effective device, exit `65`'
require dev/design/0.8.23-slice-70-tdd-plan.md 'forced unknown/OOM/allocation failure -> no effective device'
require dev/design/0.8.23-slice-70-tdd-plan.md '`DoctorGpuDiagnosticResult` maps raw probe evidence'
require dev/interfaces/cli.md '`gpu`             | `fathomdb doctor gpu [--json]`'
require dev/interfaces/cli.md '`selected_cpu_no_cuda`'
require dev/interfaces/cli.md '`cuda_not_compiled`'
require dev/interfaces/cli.md '`cuda_unavailable`'
require dev/interfaces/cli.md '`cuda_incompatible`'
require dev/interfaces/cli.md '`CudaProbeError::ProbeFailed` maps to `probe_failed`'
require dev/interfaces/cli.md 'The CLI produces `DoctorGpuDiagnosticResult` instead;'
require dev/interfaces/cli.md 'must not serialize `DeviceResolution` as the diagnostic.'
require dev/interfaces/cli.md 'does not open a database'
require dev/interfaces/cli.md 'load or download a model'
require dev/interfaces/cli.md 'write configuration'
require dev/interfaces/cli.md 'initialize an engine'
require dev/interfaces/rust.md '`visible_cuda_devices: Vec<CudaVisibleDevice>`'
require dev/interfaces/python.md '`visible_cuda_devices: tuple[CudaVisibleDevice,'
require dev/interfaces/typescript.md '`visibleCudaDevices: readonly CudaVisibleDevice[]`'
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
reject dev/design/0.8.23-slice-70-dual-runtime-device-policy.md 'The report retains the artifact CUDA toolkit/build target and driver/compute metadata as supplemental facts.'

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
reject dev/design/0.8.23-slice-20-cuda-package-rehearsal.md 'three Linux x64 consumer artifacts'

require_doctor_matrix dev/design/0.8.23-slice-70-dual-runtime-device-policy.md
require_doctor_matrix dev/interfaces/cli.md

mutation_root="$(mktemp -d)"
trap 'rm -rf "$mutation_root"' EXIT
mkdir -p "$mutation_root/dev/design" "$mutation_root/dev/interfaces"
cp dev/design/0.8.23-slice-70-dual-runtime-device-policy.md "$mutation_root/dev/design/"
cp dev/interfaces/cli.md "$mutation_root/dev/interfaces/"

assert_doctor_matrix_mutation_rejected() {
  local label="$1" file="$2" from="$3" to="$4" case_root
  case_root="$(mktemp -d "$mutation_root/case.XXXXXX")"
  cp -R "$mutation_root/dev" "$case_root/"
  local content
  content="$(<"$case_root/$file")"
  if [[ "$content" != *"$from"* ]]; then
    printf 'Slice 70 doctor-matrix fixture lacks mutation source: %s\n' "$label" >&2
    exit 1
  fi
  printf '%s\n' "${content/"$from"/"$to"}" > "$case_root/$file"
  if (require_doctor_matrix "$case_root/dev/design/0.8.23-slice-70-dual-runtime-device-policy.md" \
      && require_doctor_matrix "$case_root/dev/interfaces/cli.md") >/dev/null 2>&1; then
    printf 'Slice 70 doctor-matrix guard accepted mutation: %s\n' "$label" >&2
    exit 1
  fi
}

assert_doctor_matrix_mutation_rejected auto-no-visible-exit \
  dev/design/0.8.23-slice-70-dual-runtime-device-policy.md \
  '| `auto`, unavailable evidence | driver-presence, enumeration, or ordinal mapping | `cuda_unavailable` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `0` |' \
  '| `auto`, unavailable evidence | driver-presence, enumeration, or ordinal mapping | `cuda_unavailable` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `70` |'
assert_doctor_matrix_mutation_rejected auto-probe-failure-status \
  dev/interfaces/cli.md \
  '| `auto`, unknown, OOM, or allocation/provider failure | enumeration or mapped-device probe | `probe_failed` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `70` |' \
  '| `auto`, unknown, OOM, or allocation/provider failure | enumeration or mapped-device probe | `cuda_incompatible` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `0` |'
assert_doctor_matrix_mutation_rejected forced-incompatible-cpu \
  dev/design/0.8.23-slice-70-dual-runtime-device-policy.md \
  '| forced `cuda:N`, listed compatibility/architecture evidence | enumeration or mapped-device probe | `cuda_incompatible` | `null` | `[]` before inventory; otherwise observed inventory | `null` | `65` |' \
  '| forced `cuda:N`, listed compatibility/architecture evidence | enumeration or mapped-device probe | `cuda_incompatible` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `65` |'
assert_doctor_matrix_mutation_rejected invalid-policy-exit \
  dev/interfaces/cli.md \
  '| malformed, legacy, or otherwise invalid policy | none | `invalid_policy` | `null` | `[]` | `null` | `70` |' \
  '| malformed, legacy, or otherwise invalid policy | none | `invalid_policy` | `null` | `[]` | `null` | `0` |'

printf 'PASS  Slice 70 embedding documentation contract\n'
