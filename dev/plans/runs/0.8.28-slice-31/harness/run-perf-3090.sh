#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format diagnostics
# 0.8.28 Slice 31 PERF-03 (test-plan § 4.4): T-PERF on the RTX 3090
# (windchill3). Node addon only (OP-9 ruling): cells newP (tip) and baseP
# (slice base), CONSUMER_MODE=perf, FATHOMDB_EMBED_DEVICE=cuda:0, the RTX 3090
# selected by UUID through wc3-env.sh (never an index, never the K620); N
# processes per arm in interleaved randomised blocks. lib-host.sh runs in
# QUAL_HOST=x86-3090 mode: GPU idleness from nvidia-smi on the 3090, no nvgpu
# file-descriptor check. Analysis: analyze.py <series-root> g9x.
# Env (required): SCRATCH, GUARD_LOG, NODE_NEW and NODE_BASE (consumer dirs of
# the x86 CUDA addon built with `npm run build:native:cuda` from each commit,
# staged as an installed package). Optional: N (default 20), SEED, NODE_VERSION.
# Usage: run-perf-3090.sh
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${SCRATCH:?}" "${GUARD_LOG:?}" "${NODE_NEW:?}" "${NODE_BASE:?}"
# shellcheck source=wc3-env.sh
. "$here/wc3-env.sh" || exit 3
export QUAL_HOST=x86-3090 RTX3090_UUID
export QUAL_QUIET_STRICT=${QUAL_QUIET_STRICT:-1} NO_SWAP=${NO_SWAP:-1}
export FATHOMDB_DB_SCRATCH="$SCRATCH/dbs" FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_RERANK_DEVICE=cuda:0
out="$SCRATCH/series/g9x"
mkdir -p "$out" "$SCRATCH/dbs"
art() { find "$1/node_modules" -path '*fathomdb-linux-x64-gnu*' -name '*.node' | sed -n 1p; }
new_art=$(art "$NODE_NEW")
base_art=$(art "$NODE_BASE")
if [ -z "$new_art" ] || [ -z "$base_art" ]; then
  echo "no linux-x64-gnu addon under NODE_NEW or NODE_BASE" >&2
  exit 2
fi
{
  echo "newP node ${NODE_VERSION:-25.9.0} $NODE_NEW $new_art CONSUMER_MODE=perf"
  echo "baseP node ${NODE_VERSION:-25.9.0} $NODE_BASE $base_art CONSUMER_MODE=perf"
} >"$SCRATCH/g9x-node.tsv"
wc3_guard before-perf-03 || exit 3
flock /tmp/fathomdb-gpu.lock bash "$here/interleave.sh" "$SCRATCH/g9x-node.tsv" "${N:-20}" "${SEED:-20261313}" "$out"
rc=$?
wc3_guard after-perf-03 || exit 3
exit "$rc"
