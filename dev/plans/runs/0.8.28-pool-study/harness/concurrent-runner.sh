#!/usr/bin/env bash
# 0.8.28 pool study: R7 concurrency (protocol section 4.8). Each trial starts
# <k> `pool-consumer.mjs` processes within 100 ms of each other with the
# witness off, waits for all of them, and samples MemAvailable and swap once a
# second while they run. The caller holds the GPU lock for the whole call:
#   flock "$SCRATCH/gpu.lock" concurrent-runner.sh <variant> <k> <trials> <outdir>
# The consumer, mode and FATHOMDB_POOL_* come from the environment
# (FATHOMDB_MODULE, CONSUMER_MODE, ...). Writes <outdir>/run-<trial>-<j>.*,
# mem-<trial>.tsv (utc, MemAvailable KiB, swap used KiB) and summary.txt.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
variant=$1; k=$2; trials=$3; outdir=$4
mkdir -p "$outdir" "${FATHOMDB_DB_SCRATCH:?set FATHOMDB_DB_SCRATCH}"
# shellcheck source=/dev/null
source ~/.nvm/nvm.sh >/dev/null && nvm use 25.9.0 >/dev/null || exit 2
export FATHOMDB_POOL_VARIANT=$variant
unset FATHOMDB_GPU_ALLOCATION_WITNESS
mode0=$(power_mode)
series_header "$outdir" "variant=$variant" "k=$k" "trials=$trials" "power_mode=$mode0" \
  "consumer_sha256=$(sha256sum "$here/pool-consumer.mjs" | cut -c1-64)"
for t in $(seq 1 "$trials"); do
  host_quiet_wait "$STUDY_QUIET_WAIT_S" 2>>"$outdir/waits.txt" || { echo "REFUSED trial $t" | tee -a "$outdir/summary.txt"; exit 4; }
  pids=()
  t0=$(date +%s%N)
  for j in $(seq 1 "$k"); do
    host_json true >"$outdir/run-$t-$j.host.json"
    timeout "$STUDY_RUN_TIMEOUT_S" node "$here/pool-consumer.mjs" >"$outdir/run-$t-$j.out" 2>"$outdir/run-$t-$j.err" &
    pids+=($!)
  done
  while :; do
    alive=0
    for p in "${pids[@]}"; do [ -d "/proc/$p" ] && alive=1; done
    [ "$alive" = 0 ] && break
    printf '%s\t%s\t%s\n' "$(date -u +%T)" "$(meminfo_kib MemAvailable)" "$(swap_used_kib)" >>"$outdir/mem-$t.tsv"
    sleep 1
  done
  for j in $(seq 1 "$k"); do
    wait "${pids[$((j - 1))]}"
    rc=$?
    wall=$(( ($(date +%s%N) - t0) / 1000000 ))
    host_json null >"$outdir/run-$t-$j.host-after.json"
    python3 "$here/merge-run.py" "$outdir/run-$t-$j" "$rc" "$wall" "" >>"$outdir/summary.txt"
  done
  if ! host_floor_ok || [ "$(power_mode)" != "$mode0" ]; then
    echo "STOP floor-or-power after trial $t: $(host_json)" | tee -a "$outdir/summary.txt"; exit 3
  fi
done
echo "utc_end=$(date -u +%FT%TZ)" >>"$outdir/series-header.txt"
