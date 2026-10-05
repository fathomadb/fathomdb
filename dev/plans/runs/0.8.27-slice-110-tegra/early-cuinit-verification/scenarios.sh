#!/usr/bin/env bash
# Slice 110 early-cuInit verification: every scenario of this round, run one
# series at a time with run-series.sh. Each GPU series holds the shared GPU
# lock for its whole duration.
# Usage: scenarios.sh <worktree> <scratch> <scenario...>
#   <scratch>/consumer         installed consumer of this round's packages
#   <scratch>/consumer-nohook  installed consumer of the review fix 2 packages
#   scenarios: early, versions, late, nodeoptions, worker, small, cpu,
#   novisible, optout, cost
# LOGS (default <scratch>/logs) names the output directory.
set -u
wt=$1
scratch=$2
shift 2
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
lock="${GPU_LOCK:?set GPU_LOCK to the shared GPU lock file}"
logs="${LOGS:-$scratch/logs}"
intree="$wt/src/ts/dist/index.js"
export FATHOMDB_DB_SCRATCH="$scratch/dbs"
cuda=(FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_RERANK_DEVICE=cuda:0 FATHOMDB_GPU_ALLOCATION_WITNESS=1)
series() { # <label> <node> <count> <cwd> <env...> -- [node flag...]
  local label=$1 nodever=$2 count=$3 cwd=$4
  shift 4
  local envs=()
  while [ "$#" -gt 0 ] && [ "$1" != -- ]; do envs+=("$1"); shift; done
  [ "$#" -gt 0 ] && shift
  echo "== $label"
  flock "$lock" env "${envs[@]}" bash "$here/run-series.sh" "$nodever" "$count" "$logs/$label" "$cwd" "$@"
}
for scenario in "$@"; do
  case "$scenario" in
    early)
      for heap in 400000 1000000; do
        series "early-intree-node25-h$heap" 25.9.0 15 "$here" "${cuda[@]}" \
          FATHOMDB_MODULE="$intree" IMPORT_ORDER=early HEAP_OBJECTS="$heap"
      done ;;
    versions)
      for nodever in 24.15.0 26.10.0; do
        for heap in 400000 1000000; do
          series "early-intree-node${nodever%%.*}-h$heap" "$nodever" 5 "$here" "${cuda[@]}" \
            FATHOMDB_MODULE="$intree" IMPORT_ORDER=early HEAP_OBJECTS="$heap"
        done
      done ;;
    late)
      series late-installed-node25-h1000000 25.9.0 5 "$scratch/consumer" "${cuda[@]}" \
        FATHOMDB_MODULE=fathomdb IMPORT_ORDER=late HEAP_OBJECTS=1000000
      series late-preimport-installed-node25-h1000000 25.9.0 5 "$scratch/consumer" "${cuda[@]}" \
        FATHOMDB_MODULE=fathomdb IMPORT_ORDER=late HEAP_OBJECTS=1000000 -- --import fathomdb ;;
    nodeoptions)
      series late-nodeoptions-installed-node25-h1000000 25.9.0 3 "$scratch/consumer" "${cuda[@]}" \
        NODE_OPTIONS=--import=fathomdb FATHOMDB_MODULE=fathomdb IMPORT_ORDER=late HEAP_OBJECTS=1000000 ;;
    worker)
      series worker-first-load-intree-node25-h1000000 25.9.0 5 "$here" "${cuda[@]}" \
        CONSUMER_SCRIPT=worker-consumer.mjs FATHOMDB_MODULE="$intree" HEAP_OBJECTS=1000000 ;;
    optout)
      series optout-cost-installed-node25 25.9.0 10 "$scratch/consumer" \
        FATHOMDB_CUDA_EARLY_INIT=off CONSUMER_MODE=import FATHOMDB_MODULE=fathomdb
      series optout-small-installed-node25 25.9.0 3 "$scratch/consumer" "${cuda[@]}" \
        FATHOMDB_CUDA_EARLY_INIT=off FATHOMDB_MODULE=fathomdb ;;
    small)
      series small-intree-node25 25.9.0 10 "$here" "${cuda[@]}" FATHOMDB_MODULE="$intree"
      series small-installed-node25 25.9.0 10 "$scratch/consumer" "${cuda[@]}" FATHOMDB_MODULE=fathomdb ;;
    cpu)
      series cpu-intree-node25 25.9.0 3 "$here" FATHOMDB_EMBED_DEVICE=cpu FATHOMDB_RERANK_DEVICE=cpu \
        EXPECT_DEVICE=cpu FATHOMDB_MODULE="$intree" ;;
    novisible)
      series novisible-auto-intree-node25 25.9.0 3 "$here" CUDA_VISIBLE_DEVICES= \
        EXPECT_DEVICE=cpu FATHOMDB_MODULE="$intree" ;;
    cost)
      series cost-hook-installed-node25 25.9.0 10 "$scratch/consumer" \
        CONSUMER_MODE=import FATHOMDB_MODULE=fathomdb
      series cost-skipped-installed-node25 25.9.0 10 "$scratch/consumer" \
        FATHOMDB_EMBED_DEVICE=cpu FATHOMDB_RERANK_DEVICE=cpu CONSUMER_MODE=import FATHOMDB_MODULE=fathomdb
      series cost-nohook-installed-node25 25.9.0 10 "$scratch/consumer-nohook" \
        CONSUMER_MODE=import FATHOMDB_MODULE=fathomdb ;;
    *) echo "unknown scenario $scenario" >&2; exit 2 ;;
  esac
done
