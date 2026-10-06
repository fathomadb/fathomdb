#!/usr/bin/env bash
# 0.8.28 pool study Node series runner (protocol section 4.4). Runs <count>
# fresh `node pool-consumer.mjs` processes one at a time, each after the
# section 1.1 host-quiet check, each under `timeout $STUDY_RUN_TIMEOUT_S`, after one recorded
# throwaway warm-cache run (run-000, excluded from analysis; WARM_RUN=0 skips
# it). The caller holds the GPU lock for the series:
#   flock "$SCRATCH/gpu.lock" run-series.sh <variant> <node-version> <count> <outdir> [node flag...]
# Env from the caller: FATHOMDB_MODULE (absolute path of the installed
# package's dist/index.js), FATHOMDB_DB_SCRATCH, CONSUMER_MODE, IMPORT_ORDER,
# HEAP_OBJECTS, HEAP_GROW_AFTER_OPEN, FATHOMDB_POOL_MAXSIZE,
# FATHOMDB_POOL_RELEASE_THRESHOLD, FATHOMDB_POOL_MAPS_DIR (a directory: one
# per-run subdirectory is used), MAPS (1: the consumer snapshots maps), the
# device policy variables and the witness. SEED_MODE=perrun passes
# --random-seed=<run number> to node; SEED_MODE=mod:K passes
# --random-seed=((run-1) mod K)+1 (each seed K runs apart). RUN_FROM=<n>
# runs <count> processes numbered from <n> with no warm-up run (used by the
# interleaved-block driver). ARTIFACT names the .node under test (its
# sha256 goes into the header).
# CONSUMER_LANG=py runs "$PYTHON pool_smoke.py" instead of Node (the Node
# version argument is then unused).
# Writes run-NNN.{out,err,host.json,json} and summary.txt.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
variant=$1; nodever=$2; count=$3; outdir=$4; shift 4
mkdir -p "$outdir" "${FATHOMDB_DB_SCRATCH:?set FATHOMDB_DB_SCRATCH}"
# shellcheck source=/dev/null
source ~/.nvm/nvm.sh >/dev/null && nvm use "$nodever" >/dev/null || exit 2
export FATHOMDB_POOL_VARIANT=$variant
mode0=$(power_mode)
series_header "$outdir" "variant=$variant" "count=$count" "node_flags=$*" "seed_mode=${SEED_MODE:-none}" \
  "artifact_sha256=$( [ -n "${ARTIFACT:-}" ] && sha256sum "$ARTIFACT" | cut -c1-64)" \
  "consumer_sha256=$(sha256sum "$here/pool-consumer.mjs" | cut -c1-64)" "power_mode=$mode0" \
  "maps=${MAPS:-0}"
maps_root=${FATHOMDB_POOL_MAPS_DIR:-}
first=0; [ "${WARM_RUN:-1}" = 0 ] && first=1
last=$count
if [ -n "${RUN_FROM:-}" ]; then first=$RUN_FROM; last=$((RUN_FROM + count - 1)); fi
for n in $(seq "$first" "$last"); do
  i=$(printf '%03d' "$n")
  host_quiet_wait "$STUDY_QUIET_WAIT_S" 2>>"$outdir/waits.txt" || { echo "REFUSED run $i" | tee -a "$outdir/summary.txt"; exit 4; }
  host_json true >"$outdir/run-$i.host.json"
  flags=("$@")
  if [ "$n" -gt 0 ]; then
    case "${SEED_MODE:-}" in
      perrun) flags+=("--random-seed=$n") ;;
      mod:*) flags+=("--random-seed=$(( (n - 1) % ${SEED_MODE#mod:} + 1 ))") ;;
    esac
  fi
  run_maps=""
  if [ "${MAPS:-0}" = 1 ] || [ -n "$maps_root" ]; then run_maps="$outdir/maps/run-$i"; mkdir -p "$run_maps"; fi
  t0=$(date +%s%N)
  consumer_maps=""; [ "${MAPS:-0}" = 1 ] && consumer_maps=$run_maps
  if [ "${CONSUMER_LANG:-node}" = py ]; then
    cmd=("${PYTHON:?set PYTHON for CONSUMER_LANG=py}" "$here/pool_smoke.py")
  else
    cmd=(node "${flags[@]}" "$here/pool-consumer.mjs")
  fi
  env ${consumer_maps:+MAPS_DIR=$consumer_maps} ${maps_root:+FATHOMDB_POOL_MAPS_DIR=$run_maps} \
    timeout "$STUDY_RUN_TIMEOUT_S" "${cmd[@]}" >"$outdir/run-$i.out" 2>"$outdir/run-$i.err"
  rc=$?
  wall=$(( ($(date +%s%N) - t0) / 1000000 ))
  host_json null >"$outdir/run-$i.host-after.json"
  python3 "$here/merge-run.py" "$outdir/run-$i" "$rc" "$wall" "${flags[*]}" >>"$outdir/summary.txt"
  if ! host_floor_ok || [ "$(power_mode)" != "$mode0" ]; then
    echo "STOP floor-or-power after run $i: $(host_json)" | tee -a "$outdir/summary.txt"; exit 3
  fi
done
echo "utc_end=$(date -u +%FT%TZ)" >>"$outdir/series-header.txt"
