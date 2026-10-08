#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions here only format diagnostics; a failed read shows in the recorded data
# 0.8.28 Slice 30 qualification: host-quiet check, host record and series
# header. Sourced by every runner. Adapted from the pool study's lib-host.sh.
#
# SCRATCH comes from the environment so committed copies carry no host path.
# Runners are started under `flock /tmp/fathomdb-gpu.lock` by the caller.

: "${SCRATCH:?set SCRATCH}"
# shellcheck source=qual-config.sh
source "$(dirname "${BASH_SOURCE[0]}")/qual-config.sh"
GPU_LOAD_SYSFS=/sys/devices/platform/bus@0/17000000.gpu/load

meminfo_kib() { awk -v k="$1:" '$1 == k { print $2 }' /proc/meminfo; }

swap_used_kib() { echo $(($(meminfo_kib SwapTotal) - $(meminfo_kib SwapFree))); }

# The swap in use when the first runner of a series sources this file; exported
# so nested runners keep it. Only a NO_SWAP=1 series stops on a rise.
SWAP_BASE_KIB="${SWAP_BASE_KIB:-$(swap_used_kib)}"
export SWAP_BASE_KIB
NO_SWAP="${NO_SWAP:-0}"
swap_grew() { [ "$(swap_used_kib)" -gt $((SWAP_BASE_KIB + QUAL_SWAP_TOLERANCE_KIB)) ]; }

# Compilers, pytest and other consumers refuse a run (names matched exactly).
busy_procs() {
  ps -eo pid=,comm=,args= | awk -v self="$$" '
    $1 == self { next }
    $2 ~ /^(cargo|rustc|nvcc|maturin|cc1|cc1plus|cicc|ptxas)$/ { print $1 ":" $2; next }
    /[p]ytest/ { print $1 ":py-test"; next }
    ($2 ~ /^(node|python|python3)/) && /([p]ool-consumer|[p]ool_smoke)/ { print $1 ":consumer" }
  ' | tr '\n' ' '
}

# Same-user processes holding a GPU device node open: CUDA work started
# without the GPU lock.
gpu_fd_holders() {
  find /proc/[0-9]*/fd -maxdepth 1 \( -lname '*nvgpu*' -o -lname '*nvhost*' -o -lname '*nvmap*' \) 2>/dev/null |
    awk -F/ -v self="$$" '$3 != self { seen[$3] = 1 } END { for (p in seen) printf "%s ", p }'
}

gpu_load() { cat "$GPU_LOAD_SYSFS" 2>/dev/null || echo -1; }

# One JSON object describing the host now. $1: GPU idle verdict.
host_json() {
  local idle=${1:-null}
  printf '{"utc":"%s","memAvailableKiB":%s,"swapUsedKiB":%s,"swapBaseKiB":%s,"noSwap":%s,"cachedKiB":%s,"loadAvg1":%s,"gpuLoad":%s,"gpuIdleBefore":%s,"busy":"%s"}' \
    "$(date -u +%FT%T.%3NZ)" "$(meminfo_kib MemAvailable)" "$(swap_used_kib)" "$SWAP_BASE_KIB" \
    "$NO_SWAP" "$(meminfo_kib Cached)" "$(cut -d' ' -f1 /proc/loadavg)" "$(gpu_load)" "$idle" "$(busy_procs)"
}

# Returns 0 when the host is quiet; otherwise prints the failed conditions and
# returns 1.
host_quiet_now() {
  local why="" busy load avail i g
  if [ "$QUAL_QUIET_STRICT" = 1 ]; then
    busy=$(busy_procs)
    if [ -n "$busy" ]; then why+="busy[$busy] "; fi
  fi
  busy=$(gpu_fd_holders)
  if [ -n "$busy" ]; then why+="gpu_fds[$busy] "; fi
  load=$(cut -d' ' -f1 /proc/loadavg)
  if [ "$QUAL_QUIET_STRICT" = 1 ]; then
    awk -v l="$load" -v m="$QUAL_QUIET_LOAD1_MAX" 'BEGIN { exit !(l < m) }' || why+="load1=$load "
  fi
  avail=$(meminfo_kib MemAvailable)
  if [ "$avail" -lt "$QUAL_QUIET_MEM_KIB" ]; then why+="memavail_kib=$avail "; fi
  if [ "$NO_SWAP" = 1 ] && swap_grew; then
    why+="swap_used_kib=$(swap_used_kib)>base=$SWAP_BASE_KIB "
  fi
  if [ -z "$why" ]; then
    for i in $(seq 1 "$QUAL_GPU_IDLE_SAMPLES"); do
      g=$(gpu_load)
      if [ "$g" != 0 ]; then why+="gpu_load=$g "; break; fi
      if [ "$i" -lt "$QUAL_GPU_IDLE_SAMPLES" ]; then sleep 1; fi
    done
  fi
  if [ -z "$why" ]; then return 0; fi
  echo "$why"
  return 1
}

# Waits (polling every 15 s) until the host is quiet, for at most $1 seconds.
host_quiet_wait() {
  local max=${1:-$QUAL_QUIET_WAIT_S} start why
  start=$(date +%s)
  while ! why=$(host_quiet_now); do
    if [ $(($(date +%s) - start)) -ge "$max" ]; then
      echo "REFUSED host not quiet after ${max}s: $why" >&2
      return 1
    fi
    echo "WAIT $(date -u +%T) $why" >&2
    sleep 15
  done
  return 0
}

# MemAvailable below the floor stops every series; swap use above the baseline
# stops a NO_SWAP=1 series.
host_floor_ok() {
  if [ "$(meminfo_kib MemAvailable)" -lt "$QUAL_MEM_FLOOR_KIB" ]; then return 1; fi
  if [ "$NO_SWAP" = 1 ] && swap_grew; then return 1; fi
  return 0
}

power_mode() { nvpmodel -q 2>/dev/null | sed -n 1p; }

# series-header.txt. $1: outdir; remaining args: key=value lines.
series_header() {
  local outdir=$1 kv
  shift
  {
    echo "utc_start=$(date -u +%FT%TZ)"
    echo "l4t=$(sed -n 1p /etc/nv_tegra_release)"
    echo "kernel=$(uname -r)"
    echo "swap_base_kib=$SWAP_BASE_KIB"
    echo "no_swap=$NO_SWAP"
    echo "power_mode=$(power_mode)"
    for kv in "$@"; do echo "$kv"; done
    env | grep -E '^(FATHOMDB_|CUDA_|NODE_OPTIONS=|HEAP_|CONSUMER_|IMPORT_)' | sort
  } >"$outdir/series-header.txt"
}
