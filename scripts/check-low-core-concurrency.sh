#!/usr/bin/env bash
# scripts/check-low-core-concurrency.sh — 2-core constrained concurrency stress check.
#
# Runs concurrency-sensitive engine tests pinned to 2 CPU cores (`taskset -c 0,1`)
# on Linux to reproduce hosted 2-core CI preemption schedules locally before tag cut.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  printf 'usage: %s [--iterations N] [--cpus LIST] [--target TEST_TARGET] [--filter TEST_FILTER] [--dry-run] [--require-taskset]\n' \
    "$(basename "$0")" >&2
}

iterations=3
cpus='0,1'
dry_run=0
require_taskset=0
filter=''
targets=()

while [ "$#" -gt 0 ]; do
  case "$1" in
    --iterations)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      iterations="$2"
      shift 2
      ;;
    --cpus)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      cpus="$2"
      shift 2
      ;;
    --target)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      targets+=("$2")
      shift 2
      ;;
    --filter)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      filter="$2"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --require-taskset)
      require_taskset=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      printf 'check-low-core-concurrency: unknown argument: %s\n' "$1" >&2
      usage
      exit 2
      ;;
  esac
done

if ! [[ "$iterations" =~ ^[1-9][0-9]*$ ]]; then
  printf 'check-low-core-concurrency: --iterations must be a positive integer (got %s)\n' "$iterations" >&2
  exit 2
fi

if ! [[ "$cpus" =~ ^[0-9]+(,[0-9]+)*$ ]]; then
  printf 'check-low-core-concurrency: --cpus must be a comma-separated CPU list (got %s)\n' "$cpus" >&2
  exit 2
fi

if [ "${#targets[@]}" -eq 0 ]; then
  targets=(
    "slice41_frozen_read_stability"
    "slice40_status_cache"
  )
fi

prefix=()
if [ "$(uname -s)" = "Linux" ] && command -v taskset >/dev/null 2>&1; then
  prefix=(taskset -c "$cpus")
elif [ "$require_taskset" -eq 1 ]; then
  printf 'check-low-core-concurrency: taskset is required under --require-taskset but unavailable\n' >&2
  exit 1
else
  printf 'check-low-core-concurrency: WARN taskset unavailable; running without CPU pinning\n' >&2
fi

for ((iter = 1; iter <= iterations; iter++)); do
  for target in "${targets[@]}"; do
    cmd=("${prefix[@]}" cargo test --locked --offline -p fathomdb-engine --features "operator test-hooks" --test "$target")
    if [ -n "$filter" ]; then
      cmd+=("$filter")
    fi
    if [ "$dry_run" -eq 1 ]; then
      printf '[iter %d/%d] %s\n' "$iter" "$iterations" "${cmd[*]}"
    else
      printf 'check-low-core-concurrency: [iter %d/%d] running %s\n' "$iter" "$iterations" "$target"
      (
        cd "$REPO_ROOT"
        "${cmd[@]}"
      )
    fi
  done
done

printf 'check-low-core-concurrency: ok (%d iteration(s), %d target(s), cpus=%s)\n' \
  "$iterations" "${#targets[@]}" "$cpus"
