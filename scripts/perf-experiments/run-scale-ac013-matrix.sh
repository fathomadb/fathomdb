#!/usr/bin/env bash
# Execute 3 scale points × 2 treatments × 5 independent AC-013 processes.
set -euo pipefail
: "${SCALE_OUTPUT_DIR:?SCALE_OUTPUT_DIR is required}"
if [ -e "$SCALE_OUTPUT_DIR" ]; then
  printf 'refusing to reuse matrix output directory: %s\n' "$SCALE_OUTPUT_DIR" >&2
  exit 2
fi
mkdir -p "$SCALE_OUTPUT_DIR"
runner="${AC013_RUNNER:-scripts/perf-experiments/run-ac013.sh}"
for rows in 10000 100000 1000000; do
  for treatment in process_cold warm; do
    for repetition in 1 2 3 4 5; do
      log="$SCALE_OUTPUT_DIR/ac013-${rows}-${treatment}-rep${repetition}.log"
      AGENT_LONG=1 AC013_VECTOR_DIM=384 AC013_CORPUS_N="$rows" \
        AC013_SCALE_TREATMENT="$treatment" LOG_PATH="$log" \
        bash "$runner"
      sha256sum "$log" >"$log.sha256"
      sha256sum -c "$log.sha256"
    done
  done
done
