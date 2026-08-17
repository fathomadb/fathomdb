#!/usr/bin/env bash
# Direct harness coverage for the Slice 40 matrix executor.  The fake
# treatment runner represents the retained record protocol, while the real
# AC-013 runner is asserted separately to keep Engine::search as the SUT.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fake="$tmp/fake-ac013.sh"
calls="$tmp/calls"

cat >"$fake" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s %s %s\n' "$AC013_CORPUS_N" "$AC013_SCALE_TREATMENT" "$LOG_PATH" >>"$CALLS"
if [ "$AC013_SCALE_TREATMENT" = process_cold ]; then
  printf 'AC013_TREATMENT_RECORD treatment=process_cold n=%s seed_write_ms=1 embedding_ms=0 projection_drain_ms=1 accepted_writes=%s vector_rows_after_drain=%s drain_outcome=ok samples_us=7 result_counts=1 query_errors=0 query_timeouts=0 query_skips=0 query_invariant_failures=0\n' "$AC013_CORPUS_N" "$AC013_CORPUS_N" "$AC013_CORPUS_N" >"$LOG_PATH"
else
  samples="$(awk 'BEGIN { for (i = 1; i <= 1000; i++) printf "%s%s", (i == 1 ? "" : ","), 7 }')"
  printf 'AC013_TREATMENT_RECORD treatment=warm n=%s seed_write_ms=1 embedding_ms=0 projection_drain_ms=1 accepted_writes=%s vector_rows_after_drain=%s drain_outcome=ok samples_us=%s result_counts=%s query_errors=0 query_timeouts=0 query_skips=0 query_invariant_failures=0\n' "$AC013_CORPUS_N" "$AC013_CORPUS_N" "$AC013_CORPUS_N" "$samples" "$samples" >"$LOG_PATH"
fi
EOF
chmod +x "$fake"

AC013_V2_TEST_MODE=1 SCALE_OUTPUT_DIR="$tmp/out" AC013_RUNNER="$fake" CALLS="$calls" \
  bash "$ROOT/scripts/perf-experiments/run-scale-ac013-matrix.sh"

mapfile -t call_lines <"$calls"
logs=("$tmp/out"/*.log)
sidecars=("$tmp/out"/*.log.sha256)
[ "${#call_lines[@]}" -eq 30 ]
[ "${#logs[@]}" -eq 30 ]
[ "${#sidecars[@]}" -eq 30 ]
for rows in 10000 100000 1000000; do
  for treatment in process_cold warm; do
    awk -v r="$rows" -v t="$treatment" '$1 == r && $2 == t { count++ } END { exit count != 5 }' "$calls"
  done
done
for log in "$tmp/out"/*process_cold*.log; do
  grep -q 'treatment=process_cold' "$log"
  record="$(<"$log")"
  samples="${record#*samples_us=}"
  samples="${samples%% *}"
  IFS=, read -r -a values <<<"$samples"
  [ "${#values[@]}" -eq 1 ]
done
for log in "$tmp/out"/*warm*.log; do
  grep -q 'treatment=warm' "$log"
  record="$(<"$log")"
  samples="${record#*samples_us=}"
  samples="${samples%% *}"
  IFS=, read -r -a values <<<"$samples"
  [ "${#values[@]}" -eq 1000 ]
done
grep -q 'opened.engine.search' "$ROOT/src/rust/crates/fathomdb-engine/tests/perf_gates.rs"
