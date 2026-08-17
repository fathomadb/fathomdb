#!/usr/bin/env bash
# Execute 3 scale points × 2 treatments × 5 independent AC-013 processes.
set -euo pipefail
: "${SCALE_OUTPUT_DIR:?SCALE_OUTPUT_DIR is required}"
root="$(git rev-parse --show-toplevel)"
runner="${AC013_RUNNER:-$root/scripts/perf-experiments/run-ac013.sh}"
helper="$root/scripts/perf-experiments/ac013-v2.py"
python3 "$helper" begin --root "$SCALE_OUTPUT_DIR"

partial_retained_after="${AC013_V2_PARTIAL_RETAINED_AFTER:-}"
if [ -n "$partial_retained_after" ] && ! [[ "$partial_retained_after" =~ ^([1-9]|[12][0-9])$ ]]; then
  printf 'AC013_V2_PARTIAL_RETAINED_AFTER must be an integer from 1 through 29\n' >&2
  exit 2
fi
attempt=0
for rows in 10000 100000 1000000; do
  for treatment in process_cold warm; do
    for repetition in 1 2 3 4 5; do
      log="$SCALE_OUTPUT_DIR/ac013-${rows}-${treatment}-rep${repetition}.log"
      : >"$log"
      set +e
      AGENT_LONG=1 AC013_VECTOR_DIM=384 AC013_CORPUS_N="$rows" \
        AC013_SCALE_TREATMENT="$treatment" LOG_PATH="$log" \
        bash "$runner"
      child_status=$?
      set -e
      attempt=$((attempt + 1))
      if [ "$child_status" -ne 0 ]; then
        python3 "$helper" partial --root "$SCALE_OUTPUT_DIR" \
          --failed-rows "$rows" --failed-treatment "$treatment" \
          --failed-repetition "$repetition" --exit-status "$child_status"
        python3 "$helper" emit-status --root "$SCALE_OUTPUT_DIR" \
          --status ENVIRONMENT_INVALID --reason "child exited with status $child_status"
        exit "$child_status"
      fi
      if ! python3 "$helper" validate-record --root "$SCALE_OUTPUT_DIR" \
        --rows "$rows" --treatment "$treatment" --log "$log" >/dev/null; then
        record_status=65
        python3 "$helper" partial --root "$SCALE_OUTPUT_DIR" \
          --failed-rows "$rows" --failed-treatment "$treatment" \
          --failed-repetition "$repetition" --exit-status "$record_status"
        python3 "$helper" emit-status --root "$SCALE_OUTPUT_DIR" \
          --status ENVIRONMENT_INVALID --reason "child treatment record failed V2 validation"
        exit "$record_status"
      fi
      (
        cd "$SCALE_OUTPUT_DIR"
        log_base="$(basename "$log")"
        sha256sum "$log_base" >"$log_base.sha256"
        sha256sum -c "$log_base.sha256"
      )
      if [ -n "$partial_retained_after" ] && [ "$attempt" -eq "$partial_retained_after" ]; then
        python3 "$helper" partial --root "$SCALE_OUTPUT_DIR"
        python3 "$helper" emit-status --root "$SCALE_OUTPUT_DIR" \
          --status INSUFFICIENT_SAMPLES --reason "explicit retained partial matrix after $attempt repetitions"
        printf 'retained %s V2 repetitions; refusing complete-matrix seal\n' "$attempt" >&2
        exit 0
      fi
      python3 "$helper" partial --root "$SCALE_OUTPUT_DIR"
    done
  done
done
python3 "$helper" seal --root "$SCALE_OUTPUT_DIR"
python3 "$helper" emit-complete --root "$SCALE_OUTPUT_DIR"
