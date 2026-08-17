#!/usr/bin/env bash
# Static guard for the reviewed Slice 70 / Slice 10 / Slice 20 v2 CLI-artifact design.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
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
require "$SLICE70" '`tar --format=posix --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner`'
require "$SLICE70" '`gzip -n`'
require "$SLICE70" 'doctor gpu --json'
require "$SLICE70" '`doctor_output_sha256`'
require "$SLICE70" '`effective_device`'
require "$SLICE70" '`reason`'

require "$SLICE10" 'fathomdb.cuda-preflight-witness/v2'
require "$SLICE10" 'fathomdb.cuda-preflight-build-input/v2'
require "$SLICE10" 'forced-cuda-unavailable-python.json'
require "$SLICE10" 'forced-cuda-incompatible-napi.json'
require "$SLICE10" 'fathomdb.cuda-forced-device-failure/v1'
require "$SLICE10" 'model-cache-manifest.json'
require "$SLICE10" 'fathomdb.cuda-model-cache/v1'
require "$SLICE10" 'read-only `HF_HOME=/fathomdb-hf` bind mount'
require "$SLICE10" 'deterministic, committed fixtures'

require "$SLICE20" '`packages/` has exactly four'
require "$SLICE20" 'cpu-cli.json'
require "$SLICE20" 'gpu-cli.json'
require "$SLICE20" 'forced-cuda-unavailable-cli.json'
require "$SLICE20" 'forced-cuda-incompatible-cli.json'
require "$SLICE20" 'forced-cuda-incompatible-cli-stdout.json'
require "$SLICE20" '"target": "x86_64-unknown-linux-gnu"'
require "$SLICE20" 'requires that the archive has exactly one POSIX'
require "$SLICE20" 'mode `0755`'
require "$SLICE20" 'model_cache_manifest_sha256'
require "$SLICE20" 'candidate-only and no-publication boundary'

require "$PLAN" 'Slice 10 witness v2 migration'
require "$PLAN" 'Deterministic Linux CLI archive'
require "$PLAN" 'No workflow, release helper, or verifier implementation belongs'

printf 'PASS  Slice 70 CLI artifact v2 design contract\n'
