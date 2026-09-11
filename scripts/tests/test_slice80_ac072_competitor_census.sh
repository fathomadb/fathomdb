#!/usr/bin/env bash
# Prove the AC-072 runner excludes its command-substitution shell from census.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
runner="$root/scripts/perf-experiments/run-slice71-ac013-cell.sh"

grep -F 'snapshot_subshell_pid=$BASHPID' "$runner" >/dev/null
grep -F '$$,$PPID,$grandparent,$great_grandparent,$snapshot_subshell_pid' "$runner" >/dev/null
grep -F 'SLICE80_EXPECTED_SOURCE_SHA' "$runner" >/dev/null
grep -F 'SLICE80_AC072_IDENTITY source_sha=%s collector_sha256=%s scanner_sha256=%s' "$runner" >/dev/null

rows=$'13 bash bash scripts/perf-experiments/run-slice71-ac013-cell.sh . /tmp/current.log\n14 bash bash scripts/perf-experiments/run-slice71-ac013-cell.sh . /tmp/other.log\n'
found=$(printf '%s' "$rows" | PYTHONDONTWRITEBYTECODE=1 \
  python3 "$root/dev/tools/slice80_read_acceptance.py" scan-processes --exclude-pids 13)
if [ "${found%% *}" != 14 ]; then
  echo "AC-072 census must exclude only the current command-substitution shell" >&2
  exit 1
fi

echo "ok test_slice80_ac072_competitor_census"
