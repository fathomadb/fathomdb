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
cargo_log="$(mktemp)"
trap 'rm -f "$cargo_log"' EXIT

cd "$cell_worktree"

sqlite_version="${SLICE71_SQLITE_VERSION:?run the source commit SQLite runtime probe first}"
libsqlite3_sys=$(grep -A1 '^name = "libsqlite3-sys"$' Cargo.lock | tail -n 1 | tr -cd '0-9.\n')

snapshot() {
  local phase="$1" load mem swap_in swap_out temp competing grandparent great_grandparent
  load=$(awk '{print $1}' /proc/loadavg)
  mem=$(awk '/MemTotal:/{total=$2} /MemAvailable:/{available=$2} END{printf "%.3f", available*100/total}' /proc/meminfo)
  swap_in=$(awk '$1=="pswpin"{print $2}' /proc/vmstat)
  swap_out=$(awk '$1=="pswpout"{print $2}' /proc/vmstat)
  temp=$(for label in /sys/class/hwmon/hwmon*/temp*_label; do
    label_text="$(sed -n '1p' "$label" 2>/dev/null)" || true
    if [ "$label_text" = "Tctl" ]; then
      input="${label%_label}_input"
      awk '{printf "%.3f", $1/1000}' "$input"
      break
    fi
  done)
  if [ -z "$temp" ]; then
    echo "required k10temp:Tctl signal is unavailable" >&2
    exit 2
  fi
  grandparent=$(ps -o ppid= -p "$PPID" | tr -d ' ')
  great_grandparent=$(ps -o ppid= -p "$grandparent" | tr -d ' ')
  competing=$(
    snapshot_subshell_pid=$BASHPID
    ps -eo pid=,comm=,args= | PYTHONDONTWRITEBYTECODE=1 \
      python3 "$runner_root/dev/tools/slice80_read_acceptance.py" scan-processes \
        --exclude-pids "$$,$PPID,$grandparent,$great_grandparent,$snapshot_subshell_pid"
  )
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
exec >"$raw_log" 2>&1
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
