#!/usr/bin/env bash
# 0.8.28 pool study: run a C probe over a configuration file, one fresh
# process per run, each after the section 1.1 host-quiet check, each under
# `timeout $STUDY_C_TIMEOUT_S`. The caller holds the GPU lock for the whole call:
#   flock "$SCRATCH/gpu.lock" run-c-matrix.sh <binary> <cfg> <reps> <outdir>
# cfg: one configuration per line, "<label> <probe args...>"; blank lines and
# lines starting with # are ignored. Each configuration runs <reps> times
# (labels may set their own count with a "reps=N" first argument).
# Writes <outdir>/run-<label>-<rep>.log, .host.json, results.txt (RESULT lines,
# tagged), series-header.txt. Stops (exit 3) if the memory/swap floor or the
# power mode changes mid-series.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
bin=$1; cfg=$2; reps=$3; outdir=$4
mkdir -p "$outdir"
cp "$cfg" "$outdir/configs.txt"
mode0=$(power_mode)
series_header "$outdir" "probe=$(basename "$bin")" "probe_sha256=$(sha256sum "$bin" | cut -c1-64)" \
  "configs=$(basename "$cfg")" "reps=$reps" "power_mode=$mode0"
touch "$outdir/results.txt"
while read -r label args; do
  [[ -z "$label" || "$label" == \#* ]] && continue
  n=$reps
  if [[ "$args" == reps=* ]]; then n=${args%% *}; n=${n#reps=}; args=${args#* }; fi
  for r in $(seq 1 "$n"); do
    host_quiet_wait "$STUDY_QUIET_WAIT_S" 2>>"$outdir/waits.txt" || { echo "REFUSED $label#$r" | tee -a "$outdir/results.txt"; exit 4; }
    host_json true >"$outdir/run-$label-$r.host.json"
    log="$outdir/run-$label-$r.log"
    t0=$(date +%s%N)
    # shellcheck disable=SC2086
    timeout "$STUDY_C_TIMEOUT_S" "$bin" --tag "$label#$r" $args >"$log" 2>&1
    rc=$?
    echo "EXIT rc=$rc wall_ms=$(( ($(date +%s%N) - t0) / 1000000 ))" >>"$log"
    grep '^RESULT' "$log" >>"$outdir/results.txt" || echo "RESULT tag=$label#$r outcome=NORESULT rc=$rc" >>"$outdir/results.txt"
    if ! host_floor_ok || [ "$(power_mode)" != "$mode0" ]; then
      echo "STOP floor-or-power after $label#$r: $(host_json)" | tee -a "$outdir/results.txt"
      exit 3
    fi
  done
done <"$cfg"
echo "utc_end=$(date -u +%FT%TZ)" >>"$outdir/series-header.txt"
