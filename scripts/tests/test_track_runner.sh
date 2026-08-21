#!/usr/bin/env bash
# Track-runner control recurrence guard.
#
# The performance program has deliberately small orchestration: a coordinator,
# isolated track worktrees, per-track review, and durable receipts. This test
# proves the guard is live in the real tree and fails when an agent/harness
# entry point loses the binding to that control.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-track-runner.sh"
RUNNER="$REPO_ROOT/scripts/track-runner.sh"

if ! "$CHECKER" --root "$REPO_ROOT" --quiet; then
  printf 'FAIL  track-runner guard rejected the real repository\n' >&2
  exit 1
fi

if ! "$RUNNER" brief TRACE-01 >/dev/null; then
  printf 'FAIL  track-runner did not resolve TRACE-01\n' >&2
  exit 1
fi

if ! "$RUNNER" status >/dev/null; then
  printf 'FAIL  track-runner did not expose the live status board\n' >&2
  exit 1
fi

if "$RUNNER" brief NOT-A-TRACK >/dev/null 2>&1; then
  printf 'FAIL  track-runner accepted an unknown track\n' >&2
  exit 1
fi

FIXTURE="$(mktemp -d)"
trap 'rm -rf "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/dev"
cp -R "$REPO_ROOT/dev/performance-benchmarking" "$FIXTURE/dev/"
cp -R "$REPO_ROOT/experiments" "$FIXTURE/"

if ! "$CHECKER" --root "$FIXTURE" --quiet; then
  printf 'FAIL  track-runner guard rejected a complete fixture\n' >&2
  exit 1
fi

rm "$FIXTURE/experiments/AGENTS.md"
if "$CHECKER" --root "$FIXTURE" --quiet >/dev/null 2>&1; then
  printf 'FAIL  track-runner guard passed without the experiment-agent binding\n' >&2
  exit 1
fi

printf 'PASS  track-runner control is bound to PROGRAM, track plans, agents, and experiment harnesses\n'
