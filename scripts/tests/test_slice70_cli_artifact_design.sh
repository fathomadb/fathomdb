#!/usr/bin/env bash
# Static guard for the reviewed Slice 70 / Slice 10 / Slice 20 v2 CLI-artifact design.
set -euo pipefail

ROOT="${SLICE70_TEST_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

require() {
  local file="$1" needle="$2"
  if ! grep -Fq "$needle" "$file"; then
    printf 'missing required Slice 70 CLI-artifact design text in %s: %s\n' "$file" "$needle" >&2
    exit 1
  fi
}

SLICE70='dev/design/0.8.23-slice-70-dual-runtime-device-policy.md'
SLICE10='dev/design/0.8.23-slice-10-cuda-contract.md'
SLICE20='dev/design/0.8.23-slice-20-cuda-package-rehearsal.md'
PLAN='dev/design/0.8.23-slice-70-tdd-plan.md'

require "$SLICE70" 'fathomdb.cuda-package-rehearsal/v2'
require "$SLICE70" 'fathomdb-${version}-x86_64-unknown-linux-gnu.tar.gz'
require "$SLICE70" 'embed-cuda = ["default-embedder", "fathomdb/embed-cuda", "fathomdb-embedder/embed-cuda"]'
require "$SLICE70" '`cargo build --locked --release -p fathomdb-cli --features embed-cuda --target x86_64-unknown-linux-gnu`'
require "$SLICE70" '`tar --format=posix --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --pax-option=delete=atime,delete=ctime`'
require "$SLICE70" '`gzip -n`'
require "$SLICE70" 'doctor gpu --json'
require "$SLICE70" 'CLI evidence is diagnostic-only'
require "$SLICE70" 'only driverless CPU with policy unset/`auto` and forced `cuda:0` unavailable'
require "$SLICE70" 'Compatible-GPU CLI evidence and visible-incompatible classifier or'
require "$SLICE70" 'hardware observations remain `PENDING_EXTERNAL`'
require "$SLICE70" 'Python and N-API installed-artifact smokes are the'
require "$SLICE70" 'only model-operation proof'
require "$SLICE70" '`["/fathomdb-cli/fathomdb-${version}-x86_64-unknown-linux-gnu/fathomdb", "doctor", "gpu", "--json"]`'
require "$SLICE70" '`doctor_output_sha256`'
require "$SLICE70" '`effective_device`'
require "$SLICE70" '`reason`'
require "$SLICE70" 'Current state enumerates visible CUDA devices'
require "$SLICE70" '`CudaProvider::enumerate_visible_cuda_devices`'
require "$SLICE70" '`cuDeviceGetUuid`'
require "$SLICE70" 'deterministic `FixtureCudaProvider`'
require "$SLICE70" 'inventory/probe fixtures'

require "$SLICE10" 'fathomdb.cuda-preflight-witness/v2'
require "$SLICE10" 'fathomdb.cuda-preflight-build-input/v2'
require "$SLICE10" 'forced-cuda-unavailable-python.json'
require "$SLICE10" 'fathomdb.cuda-forced-device-failure/v1'
require "$SLICE10" 'fathomdb.cuda-forced-device-capture/v1'
require "$SLICE10" '`provenance` is `installed_candidate` for a real record and `deterministic_fixture_provider` for a fixture'
require "$SLICE10" 'Real sealed witnesses contain only outcomes the installed candidate actually produced'
require "$SLICE10" 'incompatible records are committed verifier fixtures, not trusted-runner observations'
require "$SLICE10" 'model-cache-manifest.json'
require "$SLICE10" 'fathomdb.cuda-model-cache/v1'
require "$SLICE10" 'smoke-cache-topology.json'
require "$SLICE10" 'fathomdb.cuda-smoke-cache-topology/v1'
require "$SLICE10" 'read-only `HF_HOME=/fathomdb-hf` bind mount'
require "$SLICE10" 'separate fresh'
require "$SLICE10" 'writable `XDG_CACHE_HOME=/fathomdb-product-cache`'
require "$SLICE10" 'forced-cuda-unavailable-python-stdout.txt'
require "$SLICE10" 'forced-cuda-unavailable-python-stderr.txt'
require "$SLICE10" 'forced-cuda-unavailable-napi-stdout.txt'
require "$SLICE10" 'forced-cuda-unavailable-napi-stderr.txt'
require "$SLICE10" 'forced-cuda-incompatible-python-stdout.txt'
require "$SLICE10" 'forced-cuda-incompatible-python-stderr.txt'
require "$SLICE10" 'forced-cuda-incompatible-napi-stdout.txt'
require "$SLICE10" 'forced-cuda-incompatible-napi-stderr.txt'
require "$SLICE10" 'nonempty, non-symlink file'
require "$SLICE10" '`stdout_filename`'
require "$SLICE10" '`stderr_filename`'
require "$SLICE10" '`evidence_sha256` map binds each named capture with the same digest'
require "$SLICE10" '`visible_ordinal`, `uuid`, `name`, and `compute_capability`'
require "$SLICE10" 'ordinal is exactly `0`'
require "$SLICE10" 'selected UUID equals the UUID reported by `nvidia-smi --query-gpu=uuid`'
require "$SLICE10" '`process_id` equals `nvidia_smi_compute_process_id`'
require "$SLICE10" '`["/opt/python/cp311-cp311/bin/python", "/fathomdb-harness/forced-python-open.py"]`'
require "$SLICE10" '`["node", "/fathomdb-harness/forced-napi-open.mjs"]`'
require "$SLICE10" '`sha256("<repository>@<revision>")[..12]`'
require "$SLICE10" '`HUGGINGFACE_HUB_CACHE`'
require "$SLICE10" 'deterministic, committed fixtures'

require "$SLICE20" '`packages/` has exactly four'
require "$SLICE20" 'cpu-cli.json'
require "$SLICE20" 'forced-cuda-unavailable-cli.json'
require "$SLICE20" 'two exact CLI evidence classes'
require "$SLICE20" '`PENDING_EXTERNAL`'
require "$SLICE20" 'deterministic classifier'
require "$SLICE20" 'exact snapshot inventory, no extras'
require "$SLICE20" '"target": "x86_64-unknown-linux-gnu"'
require "$SLICE20" 'requires that the archive has exactly one POSIX'
require "$SLICE20" 'mode `0755`'
require "$SLICE20" 'model_cache_manifest_sha256'
require "$SLICE20" '`["/fathomdb-cli/fathomdb-${version}-x86_64-unknown-linux-gnu/fathomdb", "doctor", "gpu", "--json"]`'
require "$SLICE20" 'CLI evidence is'
require "$SLICE20" 'diagnostic-only: it never opens a database'
require "$SLICE20" 'fresh writable'
require "$SLICE20" '`XDG_CACHE_HOME=/fathomdb-product-cache` product cache'
require "$SLICE20" '`--pax-option=delete=atime,delete=ctime`'
require "$SLICE20" 'candidate-only and no-publication boundary'

require "$PLAN" 'Slice 10 witness v2 migration'
require "$PLAN" 'Deterministic Linux CLI archive'
require "$PLAN" 'No workflow, release helper, or verifier implementation belongs'

# Mutation cases prove this guard fails closed when a reviewed invariant is
# deleted.  The recursive invocation uses a four-file miniature tree so no
# source, workflow, release helper, or retained evidence is created.
if [[ "${SLICE70_SKIP_MUTATIONS:-0}" != 1 ]]; then
  mutation_root="$(mktemp -d)"
  trap 'rm -rf "$mutation_root"' EXIT
  mkdir -p "$mutation_root/dev/design"
  cp "$SLICE70" "$SLICE10" "$SLICE20" "$PLAN" "$mutation_root/dev/design/"

  assert_mutation_rejected() {
    local label="$1" file="$2" expression="$3"
    local case_root
    case_root="$(mktemp -d "$mutation_root/case.XXXXXX")"
    cp -R "$mutation_root/dev" "$case_root/"
    sed -i "$expression" "$case_root/dev/design/$file"
    if SLICE70_TEST_ROOT="$case_root" SLICE70_SKIP_MUTATIONS=1 "$0" >/dev/null 2>&1; then
      printf 'Slice 70 CLI-artifact guard accepted mutation: %s\n' "$label" >&2
      exit 1
    fi
  }

  assert_mutation_rejected diagnostic-only 0.8.23-slice-20-cuda-package-rehearsal.md \
    's/diagnostic-only: it never opens a database/model-operation: it opens a database/'
  assert_mutation_rejected forced-capture-inventory 0.8.23-slice-10-cuda-contract.md \
    's/forced-cuda-unavailable-python-stdout.txt/removed-forced-capture.txt/'
  assert_mutation_rejected real-vs-fixture-provenance 0.8.23-slice-10-cuda-contract.md \
    's/incompatible records are committed verifier fixtures, not trusted-runner observations/incompatible records are trusted-runner observations/'
  assert_mutation_rejected cache-topology-record 0.8.23-slice-10-cuda-contract.md \
    's/fathomdb.cuda-smoke-cache-topology\/v1/fathomdb.cuda-smoke-cache-topology\/removed/'
  assert_mutation_rejected pid-device-binding 0.8.23-slice-10-cuda-contract.md \
    's/process_id` equals `nvidia_smi_compute_process_id/process_id` differs from `nvidia_smi_compute_process_id/'
  assert_mutation_rejected cache-prefix-algorithm 0.8.23-slice-10-cuda-contract.md \
    's/sha256("<repository>@<revision>")\[\.\.12\]/sha256("<revision>@<repository>")[..12]/'
  assert_mutation_rejected provider-enumeration 0.8.23-slice-70-dual-runtime-device-policy.md \
    's/CudaProvider::enumerate_visible_cuda_devices/CudaProvider::probe_cuda/'
  assert_mutation_rejected cache-topology 0.8.23-slice-20-cuda-package-rehearsal.md \
    's/XDG_CACHE_HOME=\/fathomdb-product-cache/XDG_CACHE_HOME=\/tmp/'
  assert_mutation_rejected pax-metadata 0.8.23-slice-70-dual-runtime-device-policy.md \
    's/--pax-option=delete=atime,delete=ctime/--pax-option=preserve=atime,ctime/'
fi

printf 'PASS  Slice 70 CLI artifact v2 design contract\n'
