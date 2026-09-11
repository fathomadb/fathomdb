#!/usr/bin/env bash
# Run one identity-bound Slice 80.n AC-072 observation from a prebuilt test binary.
set -euo pipefail

if [ "$#" -ne 8 ]; then
  echo "usage: $0 WORKTREE BINARY RAW_LOG SOURCE_SHA BINARY_SHA256 INPUT_SHA256 PURPOSE CORPUS_N" >&2
  exit 2
fi

cell_worktree="$1"
binary="$2"
raw_log="$3"
source_sha="$4"
expected_binary_sha="$5"
expected_input_sha="$6"
purpose="$7"
corpus_n="$8"
runner_root=$(cd "$(dirname "$0")/../.." && pwd)

case "$purpose:$corpus_n" in
  smoke:10|acceptance:10000) ;;
  *) echo "purpose and corpus size must be smoke:10 or acceptance:10000" >&2; exit 2 ;;
esac
if [ -e "$raw_log" ] || [ ! -d "$(dirname "$raw_log")" ]; then
  echo "raw log must have a new path under a pre-created campaign directory" >&2
  exit 2
fi

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
dirty_inputs=$(git status --porcelain -- \
  Cargo.toml Cargo.lock .cargo/config.toml \
  src/rust/crates/fathomdb-engine/Cargo.toml \
  src/rust/crates/fathomdb-engine/src \
  src/rust/crates/fathomdb-engine/tests/perf_gates.rs \
  src/rust/crates/fathomdb-engine/tests/reader_pool.rs \
  src/rust/crates/fathomdb-query src/rust/crates/fathomdb-schema \
  src/rust/crates/fathomdb-embedder src/rust/crates/fathomdb-embedder-api)
if [ "$actual_source_sha" != "$source_sha" ] || [ "$actual_binary_sha" != "$expected_binary_sha" ] || \
  [ "$actual_input_sha" != "$expected_input_sha" ] || [ -n "$dirty_inputs" ]; then
  echo "Slice 80 AC-072 identity drift" >&2
  exit 2
fi
runner_sha=$(sha256sum "$runner_root/scripts/perf-experiments/run-slice80-ac072-cell.sh" | awk '{print $1}')
scanner_sha=$(sha256sum "$runner_root/dev/tools/slice80_read_acceptance.py" | awk '{print $1}')

snapshot() {
  local phase="$1" load memory swap_in swap_out temperature competing affinity cgroup_path quota quota_root grandparent great_grandparent snapshot_subshell_pid
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
  quota_root="/sys/fs/cgroup${cgroup_path}"
  while [ "$quota_root" != "/sys/fs/cgroup" ] && [ ! -r "$quota_root/cpu.max" ]; do quota_root=$(dirname "$quota_root"); done
  quota=$(sed -n '1p' "$quota_root/cpu.max" 2>/dev/null || true)
  grandparent=$(ps -o ppid= -p "$PPID" | tr -d ' ')
  great_grandparent=$(ps -o ppid= -p "$grandparent" | tr -d ' ')
  competing=$(snapshot_subshell_pid=$BASHPID
    ps -eo pid=,comm=,args= | PYTHONDONTWRITEBYTECODE=1 python3 "$runner_root/dev/tools/slice80_read_acceptance.py" scan-processes \
      --exclude-pids "$$,$PPID,$grandparent,$great_grandparent,$snapshot_subshell_pid")
  python3 - "$phase" "$load" "$memory" "$swap_in" "$swap_out" "$temperature" "$competing" "$affinity" "$quota" <<'PY'
import glob, json, os, pathlib, sys
phase, load, memory, swap_in, swap_out, temperature, competing, affinity, quota = sys.argv[1:]
def values(pattern):
    result = []
    for path in sorted(glob.glob(pattern)):
        try: result.append(pathlib.Path(path).read_text().strip())
        except OSError: pass
    return result
print("SLICE71_ENV " + json.dumps({
    "phase": phase, "load_1m": float(load), "online_cpus": len(os.sched_getaffinity(0)),
    "available_memory_percent": float(memory), "pswpin": int(swap_in), "pswpout": int(swap_out),
    "cpu_temp_c": float(temperature), "thermal_signal": "k10temp:Tctl",
    "competing_processes": competing.splitlines() if competing else [], "cpu_affinity": affinity,
    "cpu_quota": quota, "scaling_governors": sorted(set(values("/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"))),
    "scaling_frequencies_khz": [int(value) for value in values("/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq")],
}, sort_keys=True))
PY
}

set -C
: >"$raw_log"
set +C
exec >"$raw_log" 2>&1
printf 'SLICE80_AC072_IDENTITY source_sha=%s binary_sha256=%s input_sha256=%s runner_sha256=%s scanner_sha256=%s mode=performance purpose=%s selector=ac_013_vector_retrieval_latency corpus_n=%s vector_dim=384 requested_samples=1000 compiled_samples=1000 treatment=warm\n' \
  "$source_sha" "$actual_binary_sha" "$actual_input_sha" "$runner_sha" "$scanner_sha" "$purpose" "$corpus_n"
printf 'SLICE80_AC072_INVOKE AGENT_LONG=1 AC013_CORPUS_N=%s AC013_VECTOR_DIM=384 AC013_SAMPLES=1000 AC013_SCALE_TREATMENT=warm\n' "$corpus_n"
snapshot start
if [ "${SLICE80_COLLECTOR_ONLY:-0}" = "1" ]; then
  snapshot end
  printf 'SLICE80_AC072_TEST_EXIT status=0\n'
  exit 0
fi
child_pid=""
terminate_child() {
  if [ -n "$child_pid" ]; then
    kill -TERM "$child_pid" 2>/dev/null || true
    wait "$child_pid" 2>/dev/null || true
  fi
  exit 124
}
trap terminate_child INT TERM
set +e
AGENT_LONG=1 AC013_CORPUS_N="$corpus_n" AC013_VECTOR_DIM=384 AC013_SAMPLES=1000 AC013_SCALE_TREATMENT=warm \
  "$binary" --exact ac_013_vector_retrieval_latency --nocapture --test-threads=1 &
child_pid=$!
wait "$child_pid"
test_status=$?
child_pid=""
set -e
snapshot end
printf 'SLICE80_AC072_TEST_EXIT status=%s\n' "$test_status"
exit "$test_status"
