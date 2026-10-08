#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format diagnostics
# 0.8.28 Slice 30 qualification: concurrency (G7) and soak (G8). Each trial
# starts one process per <lang> argument (node or py) within 100 ms of each
# other, waits for all of them, and samples MemAvailable and swap once a
# second while they run. The caller holds the GPU lock for the whole call:
#   flock /tmp/fathomdb-gpu.lock concurrent-runner.sh <trials> <outdir> <lang>...
# The consumer, mode and FATHOMDB_* settings come from the environment
# (FATHOMDB_MODULE, PYTHON, CONSUMER_MODE, SOAK_SECONDS, ...). Writes
# <outdir>/run-<trial>-<j>.*, mem-<trial>.tsv (utc, MemAvailable KiB, swap used
# KiB) and summary.txt. A trial stops the series when MemAvailable falls
# below the floor while it runs (the consumers also stop themselves in soak).
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
trials=$1
outdir=$2
shift 2
langs=("$@")
k=${#langs[@]}
mkdir -p "$outdir" "${FATHOMDB_DB_SCRATCH:?set FATHOMDB_DB_SCRATCH}"
# shellcheck source=/dev/null
source ~/.nvm/nvm.sh >/dev/null && nvm use "${NODE_VERSION:-25.9.0}" >/dev/null || exit 2
mode0=$(power_mode)
series_header "$outdir" "k=$k" "langs=${langs[*]}" "trials=$trials" "power_mode=$mode0" \
  "node=$(node --version)"
for t in $(seq 1 "$trials"); do
  host_quiet_wait "$QUAL_QUIET_WAIT_S" 2>>"$outdir/waits.txt" || {
    echo "REFUSED trial $t" | tee -a "$outdir/summary.txt"
    exit 4
  }
  pids=()
  t0=$(date +%s%N)
  for j in $(seq 1 "$k"); do
    host_json true >"$outdir/run-$t-$j.host.json"
    if [ "${langs[$((j - 1))]}" = py ]; then
      timeout "$QUAL_RUN_TIMEOUT_S" "${PYTHON:?set PYTHON}" "$here/pool_smoke.py" >"$outdir/run-$t-$j.out" 2>"$outdir/run-$t-$j.err" &
    else
      timeout "$QUAL_RUN_TIMEOUT_S" node --expose-gc "$here/pool-consumer.mjs" >"$outdir/run-$t-$j.out" 2>"$outdir/run-$t-$j.err" &
    fi
    pids+=($!)
  done
  low=0
  while :; do
    alive=0
    for p in "${pids[@]}"; do
      if [ -d "/proc/$p" ]; then alive=1; fi
    done
    if [ "$alive" = 0 ]; then break; fi
    printf '%s\t%s\t%s\n' "$(date -u +%T)" "$(meminfo_kib MemAvailable)" "$(swap_used_kib)" >>"$outdir/mem-$t.tsv"
    if ! host_floor_ok; then
      low=1
      echo "FLOOR trial $t: killing consumers" | tee -a "$outdir/summary.txt"
      kill "${pids[@]}" 2>/dev/null
      break
    fi
    sleep 1
  done
  for j in $(seq 1 "$k"); do
    wait "${pids[$((j - 1))]}"
    rc=$?
    wall=$((($(date +%s%N) - t0) / 1000000))
    host_json null >"$outdir/run-$t-$j.host-after.json"
    python3 "$here/merge-run.py" "$outdir/run-$t-$j" "$rc" "$wall" "" >>"$outdir/summary.txt"
  done
  if [ "$low" = 1 ] || [ "$(power_mode)" != "$mode0" ]; then
    echo "STOP floor-or-power after trial $t: $(host_json)" | tee -a "$outdir/summary.txt"
    exit 3
  fi
done
echo "utc_end=$(date -u +%FT%TZ)" >>"$outdir/series-header.txt"
