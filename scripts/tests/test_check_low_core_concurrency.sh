#!/usr/bin/env bash
# TDD unit tests for scripts/check-low-core-concurrency.sh.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/check-low-core-concurrency.sh"

assert_fail_code() {
  local expected_code="$1"
  local label="$2"
  shift 2
  local rc=0
  "$@" >/dev/null 2>&1 || rc=$?
  if [ "$rc" -ne "$expected_code" ]; then
    printf 'FAIL [%s]: expected exit %s, got %s\n' "$label" "$expected_code" "$rc" >&2
    exit 1
  fi
  printf 'PASS %s\n' "$label"
}

# Arm 1: usage validation rejects zero/non-numeric iterations and unknown flags.
assert_fail_code 2 "rejects --iterations 0" bash "$SCRIPT" --iterations 0
assert_fail_code 2 "rejects --iterations abc" bash "$SCRIPT" --iterations abc
assert_fail_code 2 "rejects unknown flag" bash "$SCRIPT" --unknown-flag

# Arm 2: --dry-run prints exact taskset -c 0,1 command for frozen-read & status-cache targets.
dry_out="$(bash "$SCRIPT" --dry-run --iterations 2)"
grep -q -- 'taskset -c 0,1' <<<"$dry_out" || {
  printf 'FAIL [dry-run taskset]: missing taskset -c 0,1 in:\n%s\n' "$dry_out" >&2
  exit 1
}
grep -q -- 'slice41_frozen_read_stability' <<<"$dry_out" || {
  printf 'FAIL [dry-run target]: missing slice41_frozen_read_stability in:\n%s\n' "$dry_out" >&2
  exit 1
}
grep -q -- 'slice40_status_cache' <<<"$dry_out" || {
  printf 'FAIL [dry-run target]: missing slice40_status_cache in:\n%s\n' "$dry_out" >&2
  exit 1
}
printf 'PASS dry-run prints 2-core taskset concurrency targets\n'

# Arm 3: --require-taskset fails closed when taskset is unavailable on PATH.
empty_bin="$(mktemp -d)"
trap 'rm -rf "$empty_bin"' EXIT
bash_bin="$(command -v bash)"
uname_bin="$(command -v uname)"
ln -s "$bash_bin" "$empty_bin/bash"
ln -s "$uname_bin" "$empty_bin/uname"
assert_fail_code 1 "fails closed when taskset missing under --require-taskset" \
  env PATH="$empty_bin" bash "$SCRIPT" --dry-run --require-taskset

# Arm 4: live 1-iteration run of the 0.8.32 preemption regression test under 2 cores.
bash "$SCRIPT" --iterations 1 --target slice41_frozen_read_stability \
  --filter concurrent_disjoint_kind_write_and_actuate_zero_drift
printf 'PASS live 1-iteration 2-core run succeeded\n'
