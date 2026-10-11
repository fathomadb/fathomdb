#!/usr/bin/env bash
# Slice 110 allocator-fix verification: N fresh Node processes, one at a time,
# each running consumer.mjs once. Callers wrap a whole series in
# `flock <scratch>/gpu.lock` (the GPU was shared with another agent).
# Usage: run-series.sh <node-version> <device: cuda|cpu> <count> <outdir> <module> [consumer-dir]
#   module: path to an in-tree dist/index.js, or "fathomdb" for an installed consumer
#   consumer-dir: directory holding the consumer.mjs to run (the installed
#   consumer copies it beside its node_modules); defaults to this directory
# Env: FIX_SCRATCH, a scratch directory for the per-run databases.
set -u
nodever=$1
device=$2
count=$3
outdir=$4
module=$5
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cwd=${6:-$here}
scratch="${FIX_SCRATCH:?set FIX_SCRATCH to a scratch directory}"
mkdir -p "$outdir" "$scratch/dbs"
# shellcheck source=/dev/null
source ~/.nvm/nvm.sh >/dev/null && nvm use "$nodever" >/dev/null || exit 2
if [ "$device" = cuda ]; then
  devenv=(FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_RERANK_DEVICE=cuda:0 FATHOMDB_GPU_ALLOCATION_WITNESS=1)
else
  devenv=(FATHOMDB_EMBED_DEVICE=cpu FATHOMDB_RERANK_DEVICE=cpu)
fi
node_version="$(node --version)"
module_name="$(basename "$module")"
started="$(date -u +%FT%TZ)"
echo "node=$node_version device=$device count=$count module=$module_name start=$started" >"$outdir/series-header.txt"
for i in $(seq -w 1 "$count"); do
  log="$outdir/run-$i.log"
  utc="$(date -u +%FT%TZ)"
  t0="$(date +%s%N)"
  {
    echo "BEGIN run=$i device=$device node=$node_version utc=$utc"
    grep -E '^(MemAvailable|CmaFree)' /proc/meminfo
    (cd "$cwd" && env "${devenv[@]}" FATHOMDB_MODULE="$module" FATHOMDB_DB_SCRATCH="$scratch/dbs" \
      node "$cwd/consumer.mjs" 2>&1)
    rc=$?
    t1="$(date +%s%N)"
    echo "END run=$i exit=$rc wall_ms=$(((t1 - t0) / 1000000))"
  } >"$log"
  summary="$(grep -o '"outcome":"[a-z]*"\|"failedStep":"[a-zA-Z]*"\|"embedSteadyMedian":[0-9.]*' "$log")"
  echo "$i ${summary//$'\n'/ }"
done
