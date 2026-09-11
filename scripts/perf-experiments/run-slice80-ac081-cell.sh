#!/usr/bin/env bash
# Run one identity-bound AC-081a/b observation from a prebuilt test executable.
set -euo pipefail

if [ "$#" -ne 6 ]; then
  echo "usage: $0 WORKTREE BINARY RAW_LOG SOURCE_SHA BINARY_SHA256 INPUT_SHA256" >&2
  exit 2
fi

cell_worktree="$1"
binary="$2"
raw_log="$3"
source_sha="$4"
expected_binary_sha="$5"
expected_input_sha="$6"

cd "$cell_worktree"
actual_source_sha=$(git rev-parse HEAD)
actual_binary_sha=$(sha256sum "$binary" | awk '{print $1}')
actual_input_sha=$(git ls-tree -r "$source_sha" -- \
  Cargo.toml Cargo.lock .cargo/config.toml \
  src/rust/crates/fathomdb-engine/Cargo.toml \
  src/rust/crates/fathomdb-engine/src \
  src/rust/crates/fathomdb-engine/tests/perf_gates.rs \
  src/rust/crates/fathomdb-engine/tests/reader_pool.rs \
  src/rust/crates/fathomdb-query src/rust/crates/fathomdb-schema \
  src/rust/crates/fathomdb-embedder src/rust/crates/fathomdb-embedder-api | sha256sum | awk '{print $1}')

if [ "$actual_source_sha" != "$source_sha" ] || \
   [ "$actual_binary_sha" != "$expected_binary_sha" ] || \
   [ "$actual_input_sha" != "$expected_input_sha" ]; then
  echo "Slice 80 identity drift" >&2
  exit 2
fi

snapshot() {
  local phase="$1" load memory swap_in swap_out temperature competing affinity cgroup_path quota
  load=$(awk '{print $1}' /proc/loadavg)
  memory=$(awk '/MemTotal:/{total=$2} /MemAvailable:/{available=$2} END{printf "%.3f", available*100/total}' /proc/meminfo)
  swap_in=$(awk '$1=="pswpin"{print $2}' /proc/vmstat)
  swap_out=$(awk '$1=="pswpout"{print $2}' /proc/vmstat)
  temperature=$(for label in /sys/class/hwmon/hwmon*/temp*_label; do
    if [ "$(sed -n '1p' "$label" 2>/dev/null)" = "Tctl" ]; then
      awk '{printf "%.3f", $1/1000}' "${label%_label}_input"
      break
    fi
  done)
  affinity=$(awk '/Cpus_allowed_list:/{print $2}' /proc/self/status)
  cgroup_path=$(awk -F: '$1=="0"{print $3}' /proc/self/cgroup)
  quota=$(sed -n '1p' "/sys/fs/cgroup${cgroup_path}/cpu.max" 2>/dev/null || true)
  competing=$(ps -eo pid=,comm=,args= | awk -v self="$$" '
    $1 != self && ($2 ~ /^(cargo|rustc|perf_gates)$/ || $0 ~ /run-ac013[.]sh/) {print}')
  python3 - "$phase" "$load" "$memory" "$swap_in" "$swap_out" "$temperature" \
    "$competing" "$affinity" "$quota" <<'PY'
import glob
import json
import pathlib
import sys

phase, load, memory, swap_in, swap_out, temperature, competing, affinity, quota = sys.argv[1:]
def values(pattern):
    result = []
    for path in sorted(glob.glob(pattern)):
        try:
            result.append(pathlib.Path(path).read_text().strip())
        except OSError:
            pass
    return result

print("SLICE80_ENV " + json.dumps({
    "phase": phase,
    "load_1m": float(load),
    "online_cpus": len(__import__("os").sched_getaffinity(0)),
    "available_memory_percent": float(memory),
    "pswpin": int(swap_in),
    "pswpout": int(swap_out),
    "cpu_temp_c": float(temperature),
    "thermal_signal": "k10temp:Tctl",
    "competing_processes": competing.splitlines() if competing else [],
    "cpu_affinity": affinity,
    "cpu_quota": quota,
    "scaling_governors": sorted(set(values("/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"))),
    "scaling_frequencies_khz": [int(value) for value in values("/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq")],
}, sort_keys=True))
PY
}

mkdir -p "$(dirname "$raw_log")"
exec >"$raw_log" 2>&1
printf 'SLICE80_IDENTITY source_sha=%s binary_sha256=%s input_sha256=%s mode=performance\n' \
  "$source_sha" "$actual_binary_sha" "$actual_input_sha"
snapshot start
set +e
AGENT_LONG=1 "$binary" --exact ac_081_absolute_read_performance --nocapture --test-threads=1
test_status=$?
set -e
snapshot end
printf 'SLICE80_TEST_EXIT status=%s\n' "$test_status"
exit "$test_status"
