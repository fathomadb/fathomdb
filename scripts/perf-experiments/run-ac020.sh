#!/usr/bin/env bash
# Compatibility-named runner for AC-081a/b, AC-020's absolute successor.
# Env knobs:
#   AGENT_LONG  — must be set to "1" or the test early-returns
#   LOG_PATH    — log file to tee output to (default ./ac020.log)
# Output: writes the raw log to $LOG_PATH; stdout includes the
# AC081_NUMBERS line with full-precision durations and fixture counts.
set -euo pipefail

LOG_PATH="${LOG_PATH:-./ac020.log}"
AGENT_LONG="${AGENT_LONG:-1}"

export AGENT_LONG
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

cargo test --release --no-run -p fathomdb-engine --test perf_gates >/dev/null

set +e
cargo test --release -p fathomdb-engine --test perf_gates -- \
  --exact ac_081_absolute_read_performance --nocapture --test-threads=1 \
  2>&1 | tee "$LOG_PATH"
status=${PIPESTATUS[0]}
set -e

grep -E '^AC081_NUMBERS ' "$LOG_PATH" || true

exit "$status"
