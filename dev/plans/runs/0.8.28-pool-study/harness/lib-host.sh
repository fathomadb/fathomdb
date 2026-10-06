#!/usr/bin/env bash
# 0.8.28 pool study: host-quiet check, host record and series header
# (protocol section 1.1 and section 0). Sourced by every runner.
#
# Paths come from the environment so that committed copies carry no host
# path: WORKTREE (the study checkout) and SCRATCH (the session scratch
# directory). The GPU lock is "$GPU_LOCK" (default "$SCRATCH/gpu.lock"); runners
# are started under `flock "$GPU_LOCK"` by the caller and hold it per series.

: "${WORKTREE:?set WORKTREE}"
: "${SCRATCH:?set SCRATCH}"
GPU_LOCK="${GPU_LOCK:-$SCRATCH/gpu.lock}"
# shellcheck source=study-config.sh
source "$(dirname "${BASH_SOURCE[0]}")/study-config.sh"
GPU_LOAD_SYSFS=/sys/devices/platform/bus@0/17000000.gpu/load

meminfo_kib() { awk -v k="$1:" '$1 == k { print $2 }' /proc/meminfo; }

swap_used_kib() { echo $(( $(meminfo_kib SwapTotal) - $(meminfo_kib SwapFree) )); }

# zram totals over every device, bytes: orig_data_size, compr_data_size,
# mem_used_total (/sys/block/zram*/mm_stat fields 1-3).
zram_json() {
  cat /sys/block/zram*/mm_stat 2>/dev/null |
    awk '{ o += $1; c += $2; u += $3 } END { printf "{\"origBytes\":%d,\"comprBytes\":%d,\"usedBytes\":%d}", o, c, u }'
}

# Swap (section 1.1 item 3, owner ruling 8): swap is recorded, not required to
# be zero. The baseline is the swap in use when the series starts (the first
# runner or driver to source this file; exported so nested runners keep it).
# Only a series run with NO_SWAP=1 (the timing comparisons) refuses or stops
# when swap use rises above that baseline.
export SWAP_BASE_KIB="${SWAP_BASE_KIB:-$(swap_used_kib)}"
NO_SWAP="${NO_SWAP:-0}"
swap_grew() { [ "$(swap_used_kib)" -gt $((SWAP_BASE_KIB + STUDY_SWAP_TOLERANCE_KIB)) ]; }

# Processes that refuse a run (section 1.1 item 1). Names are matched exactly
# on the command name; pytest and FathomDB consumers are matched on argv.
busy_procs() {
  ps -eo pid=,comm=,args= | awk -v self="$$" '
    $1 == self { next }
    $2 ~ /^(cargo|rustc|nvcc|maturin|Runner\.Worker|cc1|cc1plus|cicc|ptxas)$/ { print $1 ":" $2; next }
    /[p]ytest/ { print $1 ":py-test"; next }
    ($2 ~ /^(node|python|python3)/) && /([p]ool-consumer|[s]oak-consumer|[p]ool_smoke|[p]ython_smoke|\/consumer\.mjs)/ { print $1 ":consumer" }
  ' | tr '\n' ' '
}

# Same-user processes holding a GPU device node open (a CUDA process; none
# hold one on this host when idle). Catches CUDA work started without the GPU
# lock.
gpu_fd_holders() {
  ls -l /proc/[0-9]*/fd 2>/dev/null | awk -v self="$$" '
    /^\/proc\/[0-9]+\/fd:$/ { split($0, a, "/"); pid = a[3]; next }
    pid != self && /nvgpu|nvhost|nvmap/ { seen[pid] = 1 }
    END { for (p in seen) printf "%s ", p }'
}

thermal_json() {
  local out="" t
  for z in /sys/devices/virtual/thermal/thermal_zone*/temp; do
    t=$(cat "$z" 2>/dev/null) || t=null
    out+="${out:+,}${t:-null}"
  done
  printf '[%s]' "$out"
}

gpu_load() { cat "$GPU_LOAD_SYSFS" 2>/dev/null || echo -1; }

# One JSON object describing the host now. $1: GPU idle verdict (true/false/null).
host_json() {
  local idle=${1:-null}
  printf '{"utc":"%s","memAvailableKiB":%s,"swapFreeKiB":%s,"swapTotalKiB":%s,"swapUsedKiB":%s,"swapBaseKiB":%s,"noSwap":%s,"zram":%s,"cachedKiB":%s,"cmaFreeKiB":%s,"loadAvg1":%s,"thermalMilliC":%s,"gpuLoad":%s,"gpuIdleBefore":%s,"busy":"%s"}' \
    "$(date -u +%FT%T.%3NZ)" "$(meminfo_kib MemAvailable)" "$(meminfo_kib SwapFree)" \
    "$(meminfo_kib SwapTotal)" "$(swap_used_kib)" "$SWAP_BASE_KIB" "$NO_SWAP" "$(zram_json)" \
    "$(meminfo_kib Cached)" "$(meminfo_kib CmaFree)" \
    "$(cut -d' ' -f1 /proc/loadavg)" "$(thermal_json)" "$(gpu_load)" "$idle" "$(busy_procs)"
}

# Returns 0 when every section 1.1 condition holds now, printing nothing;
# otherwise prints the failed conditions on one line and returns 1. GPU idle
# is the sysfs load counter tegrastats reports as GR3D_FREQ, sampled once a
# second for 3 seconds.
host_quiet_now() {
  local why="" busy load avail
  busy=$(busy_procs)
  [ -n "$busy" ] && why+="busy[$busy] "
  busy=$(gpu_fd_holders)
  [ -n "$busy" ] && why+="gpu_fds[$busy] "
  load=$(cut -d' ' -f1 /proc/loadavg)
  awk -v l="$load" -v m="$STUDY_QUIET_LOAD1_MAX" 'BEGIN { exit !(l < m) }' || why+="load1=$load "
  avail=$(meminfo_kib MemAvailable)
  [ "$avail" -ge "$STUDY_QUIET_MEM_KIB" ] || why+="memavail_kib=$avail "
  if [ "$NO_SWAP" = 1 ] && swap_grew; then
    why+="swap_used_kib=$(swap_used_kib)>base=$SWAP_BASE_KIB "
  fi
  if [ -z "$why" ]; then
    local i g
    for i in $(seq 1 "$STUDY_GPU_IDLE_SAMPLES"); do
      g=$(gpu_load)
      [ "$g" = 0 ] || { why+="gpu_load=$g "; break; }
      [ "$i" -lt "$STUDY_GPU_IDLE_SAMPLES" ] && sleep 1
    done
  fi
  [ -z "$why" ] && return 0
  echo "$why"
  return 1
}

# Waits (polling every 15 s) until the host is quiet, for at most $1 seconds
# (default STUDY_QUIET_WAIT_S). Writes waits to stderr. Returns 1 on refusal.
host_quiet_wait() {
  local max=${1:-$STUDY_QUIET_WAIT_S} start why
  start=$(date +%s)
  while ! why=$(host_quiet_now); do
    if [ $(( $(date +%s) - start )) -ge "$max" ]; then
      echo "REFUSED host not quiet after ${max}s: $why" >&2
      return 1
    fi
    echo "WAIT $(date -u +%T) $why" >&2
    sleep 15
  done
  return 0
}

# Abort floor during a series (section 1.2 / 1.3): MemAvailable below 8 GiB
# stops every series; swap use above the series-start baseline stops a
# NO_SWAP=1 series only (every other series records it in host_json).
host_floor_ok() {
  [ "$(meminfo_kib MemAvailable)" -ge "$STUDY_MEM_FLOOR_KIB" ] || return 1
  if [ "$NO_SWAP" = 1 ] && swap_grew; then return 1; fi
  return 0
}

# series-header.txt (section 0). $1: outdir; remaining args: free-form
# key=value lines describing the variant, pool parameters and artifacts.
series_header() {
  local outdir=$1; shift
  {
    echo "utc_start=$(date -u +%FT%TZ)"
    echo "l4t=$(head -1 /etc/nv_tegra_release)"
    echo "kernel=$(uname -r)"
    echo "node=$(command -v node >/dev/null && node --version || echo none)"
    echo "python=$(python3 --version 2>&1)"
    echo "nvpmodel=$(nvpmodel -q 2>/dev/null | tr '\n' ' ')"
    echo "jetson_clocks=$(jetson_clocks --show 2>&1 | head -1)"
    echo "swap_base_kib=$SWAP_BASE_KIB"
    echo "no_swap=$NO_SWAP"
    echo "swap_tolerance_kib=$STUDY_SWAP_TOLERANCE_KIB"
    echo "zram_start=$(zram_json)"
    for kv in "$@"; do echo "$kv"; done
    env | grep -E '^(FATHOMDB_|CUDA_|NODE_OPTIONS=)' | sort
  } >"$outdir/series-header.txt"
}

power_mode() { nvpmodel -q 2>/dev/null | head -1; }
