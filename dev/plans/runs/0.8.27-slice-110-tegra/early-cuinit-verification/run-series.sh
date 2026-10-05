#!/usr/bin/env bash
# Slice 110 early-cuInit verification: N fresh Node processes, one at a time,
# each running consumer.mjs once. Callers wrap every GPU series in
# `flock <scratch>/gpu.lock`.
# Usage: run-series.sh <node-version> <count> <outdir> <consumer-dir> [node flag...]
#   consumer-dir: where consumer.mjs runs (an installed consumer holds a copy
#   beside its node_modules; for in-tree runs, this directory).
# CONSUMER_SCRIPT names the script in consumer-dir (default consumer.mjs).
# Env passed through to the consumer: FATHOMDB_MODULE, FATHOMDB_DB_SCRATCH,
# IMPORT_ORDER, HEAP_OBJECTS, CONSUMER_MODE, EXPECT_DEVICE and the device
# policy variables, which the caller sets explicitly for every series.
set -u
nodever=$1
count=$2
outdir=$3
cwd=$4
shift 4
mkdir -p "$outdir" "${FATHOMDB_DB_SCRATCH:?set FATHOMDB_DB_SCRATCH}"
# shellcheck source=/dev/null
source ~/.nvm/nvm.sh >/dev/null && nvm use "$nodever" >/dev/null || exit 2
node_version="$(node --version)"
started="$(date -u +%FT%TZ)"
settings="$(env | grep -E '^(FATHOMDB_(EMBED|RERANK)_DEVICE|FATHOMDB_GPU_ALLOCATION_WITNESS|CUDA_VISIBLE_DEVICES|NODE_OPTIONS|FATHOMDB_CUDA_EARLY_INIT|CONSUMER_SCRIPT|IMPORT_ORDER|HEAP_OBJECTS|CONSUMER_MODE|EXPECT_DEVICE)=' || true)"
sorted="$(sort <<<"$settings")"
printf 'node=%s count=%s flags=%s start=%s\n%s\n' "$node_version" "$count" "$*" "$started" \
  "$sorted" >"$outdir/series-header.txt"
for i in $(seq -w 1 "$count"); do
  log="$outdir/run-$i.log"
  t0="$(date +%s%N)"
  (cd "$cwd" && node "$@" "$cwd/${CONSUMER_SCRIPT:-consumer.mjs}" >"$outdir/run-$i.out" 2>"$outdir/run-$i.err")
  rc=$?
  t1="$(date +%s%N)"
  stderr_lines="$(wc -l <"$outdir/run-$i.err")"
  {
    echo "BEGIN run=$i node=$node_version"
    cat "$outdir/run-$i.out"
    echo "STDERR lines=$stderr_lines"
    cat "$outdir/run-$i.err"
    echo "END run=$i exit=$rc wall_ms=$(((t1 - t0) / 1000000))"
  } >"$log"
  summary="$(grep -o '"outcome":"[a-z]*"\|"failedStep":"[a-zA-Z]*"\|"embedSteadyMedian":[0-9.]*' "$outdir/run-$i.out")"
  echo "$i exit=$rc ${summary//$'\n'/ }"
done
