#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format diagnostics
# 0.8.28 Slice 30 qualification series runner. Runs <count> fresh consumer
# processes one at a time, each after the host-quiet check and under
# `timeout $QUAL_RUN_TIMEOUT_S`, after one recorded throwaway warm-cache run
# (run-000, excluded from analysis; WARM_RUN=0 skips it). The caller holds the
# GPU lock for the series:
#   flock /tmp/fathomdb-gpu.lock run-series.sh <label> <count> <outdir> [node flag...]
# Env from the caller: CONSUMER_LANG (node|py, default node), NODE_VERSION
# (default 25.9.0), FATHOMDB_MODULE (absolute dist/index.js of the installed
# package; node), PYTHON (the venv interpreter; py), FATHOMDB_DB_SCRATCH,
# CONSUMER_MODE, IMPORT_ORDER, HEAP_OBJECTS, HEAP_GROW_AFTER_OPEN, the device
# policy variables and the FATHOMDB_POOL_* / FATHOMDB_CUDA_EARLY_INIT
# settings. ARTIFACT names the .node or .whl under test (sha256 into the
# header). RUN_FROM=<n> runs <count> processes numbered from <n> with no
# warm-up run (used by interleave.sh).
# Writes run-NNN.{out,err,host.json,json} and summary.txt.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
label=$1
count=$2
outdir=$3
shift 3
mkdir -p "$outdir" "${FATHOMDB_DB_SCRATCH:?set FATHOMDB_DB_SCRATCH}"
lang=${CONSUMER_LANG:-node}
if [ "$lang" = node ]; then
  # shellcheck source=/dev/null
  source ~/.nvm/nvm.sh >/dev/null && nvm use "${NODE_VERSION:-25.9.0}" >/dev/null || exit 2
fi
mode0=$(power_mode)
series_header "$outdir" "label=$label" "count=$count" "lang=$lang" "node_flags=$*" \
  "artifact_sha256=$([ -n "${ARTIFACT:-}" ] && sha256sum "$ARTIFACT" | cut -c1-64)" \
  "node=$(command -v node >/dev/null && node --version || echo none)" "power_mode=$mode0"
first=0
if [ "${WARM_RUN:-1}" = 0 ]; then first=1; fi
last=$count
if [ -n "${RUN_FROM:-}" ]; then
  first=$RUN_FROM
  last=$((RUN_FROM + count - 1))
fi
for n in $(seq "$first" "$last"); do
  i=$(printf '%03d' "$n")
  host_quiet_wait "$QUAL_QUIET_WAIT_S" 2>>"$outdir/waits.txt" || {
    echo "REFUSED run $i" | tee -a "$outdir/summary.txt"
    exit 4
  }
  host_json true >"$outdir/run-$i.host.json"
  if [ "$lang" = py ]; then
    cmd=("${PYTHON:?set PYTHON for CONSUMER_LANG=py}" "$here/pool_smoke.py")
  else
    cmd=(node "$@" "$here/pool-consumer.mjs")
  fi
  t0=$(date +%s%N)
  timeout "$QUAL_RUN_TIMEOUT_S" "${cmd[@]}" >"$outdir/run-$i.out" 2>"$outdir/run-$i.err"
  rc=$?
  wall=$((($(date +%s%N) - t0) / 1000000))
  host_json null >"$outdir/run-$i.host-after.json"
  python3 "$here/merge-run.py" "$outdir/run-$i" "$rc" "$wall" "$*" >>"$outdir/summary.txt"
  if ! host_floor_ok || [ "$(power_mode)" != "$mode0" ]; then
    echo "STOP floor-or-power after run $i: $(host_json)" | tee -a "$outdir/summary.txt"
    exit 3
  fi
done
echo "utc_end=$(date -u +%FT%TZ)" >>"$outdir/series-header.txt"
