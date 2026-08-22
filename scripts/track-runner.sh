#!/usr/bin/env bash
# Read-only entry point for the PROGRAM Track Runner control.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat >&2 <<'USAGE'
Usage: track-runner.sh check | status | brief TRACK-ID

  check           Verify that the Track Runner control remains wired.
  status          Verify the control, then print the live coordination board.
  brief TRACK-ID  Verify the control, then print the named track plan.

This command is a planning/preflight control. It never executes a benchmark,
starts a service, acquires a corpus, or spends a model budget.
USAGE
}

[ "$#" -ge 1 ] || { usage; exit 2; }
case "$1" in
  check)
    [ "$#" -eq 1 ] || { usage; exit 2; }
    exec "$SCRIPT_DIR/check-track-runner.sh"
    ;;
  status)
    [ "$#" -eq 1 ] || { usage; exit 2; }
    "$SCRIPT_DIR/check-track-runner.sh" --quiet
    exec sed -n '1,260p' "$SCRIPT_DIR/../dev/performance-benchmarking/TRACK-RUNNER-STATUS.md"
    ;;
  brief)
    [ "$#" -eq 2 ] || { usage; exit 2; }
    "$SCRIPT_DIR/check-track-runner.sh" --quiet
    case "$2" in
      SAFETY-01) plan="safety-01-campaign-controls.md" ;;
      TRACE-01) plan="trace-01-projection-lifecycle-integrity.md" ;;
      LOCOMO-01) plan="locomo-01-self-characterization.md" ;;
      PARENT-01) plan="parent-01-parent-child-screening.md" ;;
      SCALE-01) plan="scale-01-tc5-fidelity.md" ;;
      CORPUS-01) plan="corpus-01-gold-coverage.md" ;;
      ANSWER-01) plan="answer-01-shortlist-scoring.md" ;;
      TEMPORAL-01) plan="temporal-01-time-scoped-retrieval.md" ;;
      EXTRACT-01) plan="extract-01-semantic-memory.md" ;;
      MEMORY-01) plan="memory-01-native-mem0-comparison.md" ;;
      SCALE-02) plan="scale-02-local-first-envelope.md" ;;
      LATENT-01) plan="latent-01-late-chunking-feasibility.md" ;;
      GRAPH-01) plan="graph-01-projection-characterization.md" ;;
      GLOBAL-01) plan="global-01-native-graphrag.md" ;;
      REASON-01) plan="reason-01-native-hipporag2.md" ;;
      SEARCH-01) plan="search-01-ir-c-baseline.md" ;;
      *)
        printf 'track-runner: unknown track ID: %s\n' "$2" >&2
        exit 2
        ;;
    esac
    exec sed -n '1,240p' "$SCRIPT_DIR/../dev/performance-benchmarking/tracks/$plan"
    ;;
  *)
    usage
    exit 2
    ;;
esac
