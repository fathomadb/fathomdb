#!/usr/bin/env bash
# Run the AC-013 perf gate (vector retrieval latency).
# Env knobs:
#   AC013_CORPUS_N  — corpus row count (default 10000; 1000000 for canonical)
#   AC_FULL_SCALE   — set to "1" to honor ADR canonical scale
#   AGENT_LONG      — must be set to "1" or the test early-returns
#   AC013_SCALE_TREATMENT — process_cold or warm; invalid values fail closed
#   LOG_PATH        — log file to tee output to (default ./ac013.log)
set -euo pipefail

LOG_PATH="${LOG_PATH:-./ac013.log}"
AGENT_LONG="${AGENT_LONG:-1}"

export AGENT_LONG
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

cargo test --release --no-run -p fathomdb-engine --test perf_gates >/dev/null

set +e
cargo test --release -p fathomdb-engine --test perf_gates -- \
  --nocapture --test-threads=1 ac_013 \
  2>&1 | tee "$LOG_PATH"
status=${PIPESTATUS[0]}
set -e

grep -E '^AC013_NUMBERS ' "$LOG_PATH" || true
if [ -n "${AC013_SCALE_TREATMENT:-}" ]; then
  matches="$(awk -v treatment="$AC013_SCALE_TREATMENT" '$1 == "AC013_TREATMENT_RECORD" && $2 == "treatment=" treatment { count++ } END { print count + 0 }' "$LOG_PATH")"
  if [ "$matches" -ne 1 ]; then
    printf 'AC-013 treatment record mismatch: requested=%s matching_records=%s\n' "$AC013_SCALE_TREATMENT" "$matches" >&2
    exit 1
  fi
  grep -E "^AC013_TREATMENT_RECORD treatment=${AC013_SCALE_TREATMENT} " "$LOG_PATH"
fi

exit "$status"
