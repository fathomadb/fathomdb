#!/usr/bin/env bash
# Validate the Track Runner control that governs PROGRAM execution.
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage: check-track-runner.sh [--root PATH] [--quiet]

Checks that the performance program, its track plans, the scoped agent
instructions, and the experiment-harness instructions remain bound to the
Track Runner operating control.
USAGE
}

root=""
quiet=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --root)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      root="$2"
      shift 2
      ;;
    --quiet)
      quiet=1
      shift
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

if [ -z "$root" ]; then
  root="$(git rev-parse --show-toplevel)" || exit 1
fi
root="$(cd "$root" && pwd)" || exit 1

fail=0
require_file() {
  local path="$1"
  if [ ! -f "$root/$path" ]; then
    printf 'FAIL track-runner: required file is missing: %s\n' "$path" >&2
    fail=1
  fi
}

require_text() {
  local path="$1" needle="$2"
  if ! grep -Fq -- "$needle" "$root/$path"; then
    printf 'FAIL track-runner: %s must contain %s\n' "$path" "$needle" >&2
    fail=1
  fi
}

required_files=(
  "dev/performance-benchmarking/PROGRAM.md"
  "dev/performance-benchmarking/PROGRAM-GOALS.md"
  "dev/performance-benchmarking/TRACK-RUNNER.md"
  "dev/performance-benchmarking/TRACK-RUNNER-STATUS.md"
  "dev/performance-benchmarking/tracks/README.md"
  "dev/performance-benchmarking/AGENTS.md"
  "experiments/README.md"
  "experiments/AGENTS.md"
  "experiments/fathomdb_locomo.py"
  "experiments/mem0_oss.py"
  "experiments/mem0_comparison.py"
  "experiments/configs/mem0-oss/locomo-fathomdb-seam.example.json"
  "experiments/configs/mem0-oss/locomo-native-predict.example.json"
)
for path in "${required_files[@]}"; do
  require_file "$path"
done

if [ "$fail" -ne 0 ]; then
  exit 1
fi

require_text "dev/performance-benchmarking/PROGRAM.md" "TRACK-RUNNER.md"
require_text "dev/performance-benchmarking/PROGRAM-GOALS.md" "TRACK-RUNNER.md"
require_text "dev/performance-benchmarking/tracks/README.md" "TRACK-RUNNER.md"
require_text "dev/performance-benchmarking/AGENTS.md" "track-runner.sh"
require_text "dev/performance-benchmarking/AGENTS.md" "TRACK-RUNNER-STATUS.md"
require_text "experiments/README.md" "track-runner.sh check"
require_text "experiments/AGENTS.md" "track-runner.sh"
require_text "dev/performance-benchmarking/TRACK-RUNNER.md" "TRACK-RUNNER-STATUS.md"
require_text "dev/performance-benchmarking/TRACK-RUNNER-STATUS.md" "## Immediate sequence"
require_text "dev/performance-benchmarking/TRACK-RUNNER-STATUS.md" "## Track status"
require_text "dev/performance-benchmarking/TRACK-RUNNER-STATUS.md" "## Board rules"
require_text "experiments/fathomdb_locomo.py" "program_track"
require_text "experiments/fathomdb_locomo.py" "MEMORY-01"
require_text "experiments/mem0_oss.py" "program_track"
require_text "experiments/mem0_oss.py" "MEMORY-01"
require_text "experiments/mem0_comparison.py" "program_track"
require_text "experiments/configs/mem0-oss/locomo-fathomdb-seam.example.json" "MEMORY-01"
require_text "experiments/configs/mem0-oss/locomo-native-predict.example.json" "MEMORY-01"

track_bindings=(
  "SAFETY-01 tracks/safety-01-campaign-controls.md"
  "TRACE-01 tracks/trace-01-projection-lifecycle-integrity.md"
  "LOCOMO-01 tracks/locomo-01-self-characterization.md"
  "PARENT-01 tracks/parent-01-parent-child-screening.md"
  "SCALE-01 tracks/scale-01-tc5-fidelity.md"
  "CORPUS-01 tracks/corpus-01-gold-coverage.md"
  "ANSWER-01 tracks/answer-01-shortlist-scoring.md"
  "TEMPORAL-01 tracks/temporal-01-time-scoped-retrieval.md"
  "EXTRACT-01 tracks/extract-01-semantic-memory.md"
  "MEMORY-01 tracks/memory-01-native-mem0-comparison.md"
  "SCALE-02 tracks/scale-02-local-first-envelope.md"
  "LATENT-01 tracks/latent-01-late-chunking-feasibility.md"
  "GRAPH-01 tracks/graph-01-projection-characterization.md"
  "GLOBAL-01 tracks/global-01-native-graphrag.md"
  "REASON-01 tracks/reason-01-native-hipporag2.md"
  "SEARCH-01 tracks/search-01-ir-c-baseline.md"
)
for binding in "${track_bindings[@]}"; do
  track_id="${binding%% *}"
  plan="${binding#* }"
  require_file "dev/performance-benchmarking/$plan"
  if [ -f "$root/dev/performance-benchmarking/$plan" ]; then
    require_text "dev/performance-benchmarking/$plan" "$track_id"
  fi
  require_text "dev/performance-benchmarking/PROGRAM.md" "$track_id"
  require_text "dev/performance-benchmarking/TRACK-RUNNER.md" "$track_id"
  require_text "dev/performance-benchmarking/TRACK-RUNNER-STATUS.md" "$track_id"
done

if [ "$fail" -ne 0 ]; then
  exit 1
fi

if [ "$quiet" -eq 0 ]; then
  printf 'ok    track-runner: PROGRAM, track plans, scoped agents, and experiment harnesses are bound\n'
fi
