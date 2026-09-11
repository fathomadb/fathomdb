#!/usr/bin/env bash
# Prove both Slice 80 collectors have a benchmark-free production snapshot path.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)

for runner in \
  "$root/scripts/perf-experiments/run-slice80-ac081-cell.sh" \
  "$root/scripts/perf-experiments/run-slice71-ac013-cell.sh"; do
  grep -F 'SLICE80_COLLECTOR_ONLY' "$runner" >/dev/null
  grep -F 'snapshot start' "$runner" >/dev/null
  grep -F 'snapshot end' "$runner" >/dev/null
done
