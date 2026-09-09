#!/usr/bin/env bash
# Execute one preregistered Slice 71 AC-013 cell with auditable environment data.
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 WORKTREE RAW_LOG" >&2
  exit 2
fi

cell_worktree="$1"
raw_log="$2"
runner_root="$(git rev-parse --show-toplevel)"
runtime_probe="$(mktemp)"
probe_db="$(mktemp --suffix=.fathomdb)"
cargo_log="$(mktemp)"
rm -f "$probe_db"
trap 'rm -f "$runtime_probe" "$probe_db" "$cargo_log"' EXIT

cd "$cell_worktree"

FATHOM_SLICE45_DATABASE="$probe_db" \
FATHOM_SLICE45_ROWS=100 \
FATHOM_SLICE45_SAMPLES=1 \
FATHOM_SLICE45_MODE=latency \
FATHOM_SLICE45_LATENCY_PAIR=current_state:frozen_state \
cargo test --release -p fathomdb-engine --test slice45_pagination_performance \
  measure_slice45_pagination_overhead -- --ignored --exact --nocapture \
  >"$runtime_probe" 2>&1

sqlite_version="$(python3 - "$runtime_probe" <<'PY'
import json
import sys

for line in open(sys.argv[1], encoding="utf-8"):
    try:
        value = json.loads(line)
    except json.JSONDecodeError:
        continue
    if value.get("schema_version") == "slice45-pagination-performance.v1":
        print(value["sqlite_version"])
        break
else:
    raise SystemExit("SQLite runtime identity missing from probe")
PY
)"
libsqlite3_sys="$(awk '/^name = "libsqlite3-sys"$/{found=1; next} found && /^version = /{gsub(/"/, "", $3); print $3; exit}' Cargo.lock)"

snapshot() {
  local phase="$1" load mem swap_in swap_out temp competing
  load="$(awk '{print $1}' /proc/loadavg)"
  mem="$(awk '/MemTotal:/{total=$2} /MemAvailable:/{available=$2} END{printf "%.3f", available*100/total}' /proc/meminfo)"
  swap_in="$(awk '$1=="pswpin"{print $2}' /proc/vmstat)"
  swap_out="$(awk '$1=="pswpout"{print $2}' /proc/vmstat)"
  temp="$(for label in /sys/class/hwmon/hwmon*/temp*_label; do
    if [ "$(sed -n '1p' "$label" 2>/dev/null)" = "Tctl" ]; then
      input="${label%_label}_input"
      awk '{printf "%.3f", $1/1000}' "$input"
      break
    fi
  done)"
  if [ -z "$temp" ]; then
    echo "required k10temp:Tctl signal is unavailable" >&2
    exit 2
  fi
  competing="$(ps -eo pid=,comm=,args= | awk '$2 ~ /^(cargo|rustc|perf_gates)$/ || $0 ~ /run-ac013[.]sh/ {print}' || true)"
  python3 - "$phase" "$load" "$mem" "$swap_in" "$swap_out" "$temp" "$competing" "$sqlite_version" "$libsqlite3_sys" <<'PY'
import json
import sys

phase, load, mem, swap_in, swap_out, temp, competing, sqlite, libsqlite = sys.argv[1:]
print("SLICE71_ENV " + json.dumps({
    "phase": phase,
    "load_1m": float(load),
    "available_memory_percent": float(mem),
    "pswpin": int(swap_in),
    "pswpout": int(swap_out),
    "cpu_temp_c": float(temp),
    "thermal_signal": "k10temp:Tctl",
    "competing_processes": [line for line in competing.splitlines() if line],
    "sqlite_version": sqlite,
    "libsqlite3_sys": libsqlite,
}, sort_keys=True))
PY
}

mkdir -p "$(dirname "$raw_log")"
exec > >(tee "$raw_log") 2>&1
snapshot start
set +e
LOG_PATH="$cargo_log" AGENT_LONG=1 AC013_CORPUS_N=10000 \
  AC013_VECTOR_DIM=384 AC013_SAMPLES=1000 AC013_SCALE_TREATMENT=warm \
  CARGO_BUILD_JOBS=1 bash "$runner_root/scripts/perf-experiments/run-ac013.sh"
test_status=$?
set -e
snapshot end
printf 'SLICE71_TEST_EXIT status=%s\n' "$test_status"
if [ "$test_status" -ne 0 ] && [ "$test_status" -ne 101 ]; then
  exit "$test_status"
fi
