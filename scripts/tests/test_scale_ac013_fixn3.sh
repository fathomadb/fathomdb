#!/usr/bin/env bash
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
runner="$root/scripts/perf-experiments/run-ac013.sh"
matrix="$root/scripts/perf-experiments/run-scale-ac013-matrix.sh"
test -x "$runner"
test -x "$matrix"
grep -q 'AC013_TREATMENT_RECORD' "$runner"
grep -q 'sha256sum.*-c' "$matrix"
grep -q 'vector_rows_after_drain' "$root/src/rust/crates/fathomdb-engine/tests/perf_gates.rs"
