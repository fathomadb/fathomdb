#!/usr/bin/env bash
# 0.8.28 pool study, protocol section 6.5 (contiguous need): for each gap G
# and placement (between units, in-unit), sweep maxSize in 1 GiB steps from
# 1 GiB, 3 fresh pool_gap processes per point, until the first point with any
# failure, then two more points as confirmations (or until 48 GiB). Also:
# G = 0 negative controls, the unfenced control ceiling (48..64 GiB) with and
# without a 4 GiB device hold, and the Slice 110 control (pool_va_repro, the
# default pool created first) at 48/50/52/56 GiB.
# Usage (GPU lock held by the caller): gap-sweep.sh <bindir> <outdir>
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
bin=$1; outdir=$2
mkdir -p "$outdir"
mode0=$(power_mode)
series_header "$outdir" "probe=pool_gap" "probe_sha256=$(sha256sum "$bin/pool_gap" | cut -c1-64)" "power_mode=$mode0"
touch "$outdir/results.txt"
run_point() { # label reps probe args... ; echoes the number of failed runs
  local label=$1 reps=$2 probe=$3; shift 3
  local fails=0 r log
  for r in $(seq 1 "$reps"); do
    host_quiet_wait "$STUDY_QUIET_WAIT_S" 2>>"$outdir/waits.txt" || exit 4
    host_json true >"$outdir/run-$label-$r.host.json"
    log="$outdir/run-$label-$r.log"
    timeout "$STUDY_C_TIMEOUT_S" "$bin/$probe" "$@" --tag "$label#$r" >"$log" 2>&1
    echo "EXIT rc=$?" >>"$log"
    if grep -q '^RESULT' "$log"; then grep '^RESULT' "$log" >>"$outdir/results.txt"
    else echo "RESULT tag=$label#$r outcome=NORESULT" >>"$outdir/results.txt"; fi
    grep -q 'probe_rc=CUDA_SUCCESS\|setpool_async_4=0 ' "$log" || fails=$((fails + 1))
    if ! host_floor_ok || [ "$(power_mode)" != "$mode0" ]; then
      echo "STOP floor-or-power at $label#$r" | tee -a "$outdir/results.txt"; exit 3
    fi
  done
  echo "$fails"
}
sweep() { # label-prefix lo_gib hi_gib extra args...
  local prefix=$1 lo=$2 hi=$3; shift 3
  local ms fails after=-1
  for ms in $(seq "$lo" "$hi"); do
    fails=$(run_point "$prefix-m${ms}G" 3 pool_gap --maxsize $((ms * 1024)) "$@")
    echo "POINT $prefix maxsize=${ms}GiB fails=$fails/3" | tee -a "$outdir/points.txt"
    if [ "$after" -ge 0 ]; then after=$((after + 1)); [ "$after" -ge 2 ] && break
    elif [ "$fails" -gt 0 ]; then after=0; fi
  done
}
for g in 1 2 4 8 12 16; do
  sweep "between-g${g}G" 1 48 --gap $((g * 1024))
  sweep "inunit-g${g}G" 1 48 --gap $((g * 1024)) --in-unit
done
for mode in between inunit; do
  extra=(); [ "$mode" = inunit ] && extra=(--in-unit)
  f=$(run_point "$mode-g0-m1G" 3 pool_gap --maxsize 1024 --gap 0 "${extra[@]}")
  echo "POINT $mode-g0 maxsize=1GiB fails=$f/3" | tee -a "$outdir/points.txt"
done
sweep "nofill" 48 64 --no-fill
sweep "nofill-hold4G" 48 64 --no-fill --hold 4096
for ms in 48 50 52 56; do
  f=$(run_point "slice110-control-m${ms}G" 3 pool_va_repro --pool-maxsize "${ms}G")
  echo "POINT slice110-control (default pool first) maxsize=${ms}GiB fails=$f/3" | tee -a "$outdir/points.txt"
done
echo "utc_end=$(date -u +%FT%TZ)" >>"$outdir/series-header.txt"
