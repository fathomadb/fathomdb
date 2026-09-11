#!/usr/bin/env bash
# Verify the 80.n AC-072 route invokes only its supplied test executable.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
runner="$root/scripts/perf-experiments/run-slice80-ac072-cell.sh"

test -x "$runner"
grep -F 'ac_013_vector_retrieval_latency' "$runner" >/dev/null
if rg -n 'cargo|rustc|run-ac013.sh' "$runner"; then
  echo "prebuilt AC-072 route must not invoke a build wrapper" >&2
  exit 1
fi
grep -F 'run-slice80-ac072-cell' "$root/dev/tools/slice80_read_acceptance.py" >/dev/null

echo "ok test_slice80_ac072_prebuilt_runner"
