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
  treatment_records_file="$(mktemp)"
  if ! awk '$1 == "AC013_TREATMENT_RECORD" { print }' "$LOG_PATH" >"$treatment_records_file"; then
    rm -f "$treatment_records_file"
    exit 1
  fi
  mapfile -t treatment_records <"$treatment_records_file"
  rm -f "$treatment_records_file"
  if [ "${#treatment_records[@]}" -ne 1 ] || [[ "${treatment_records[0]:-}" != "AC013_TREATMENT_RECORD treatment=${AC013_SCALE_TREATMENT} "* ]]; then
    printf 'AC-013 treatment record mismatch: requested=%s total_records=%s\n' "$AC013_SCALE_TREATMENT" "${#treatment_records[@]}" >&2
    exit 1
  fi
  printf '%s\n' "${treatment_records[0]}"
fi

exit "$status"
