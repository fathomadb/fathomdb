#!/usr/bin/env bash
# 0.8.28 Slice 31 (AC31-07): the single-cudarc guard. One fixture per failure
# condition (A on a lock file; C three ways; D), the two exclusions passing,
# and the guard GREEN on the real tree. Fixtures: test-plan.md appendix A.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
GUARD="$REPO_ROOT/scripts/check-single-cudarc.sh"
FIXTURES="$REPO_ROOT/scripts/tests/fixtures/single-cudarc"

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

[ -f "$GUARD" ] || fail "guard script scripts/check-single-cudarc.sh is absent"

# A tree fixture that cargo rejects would make an arm fail for the wrong
# reason, so every tree fixture must be a consistent, locked workspace first.
for tree in "$FIXTURES"/tree/*/; do
  if ! (cd "$tree" && cargo metadata --offline --locked --format-version 1 >/dev/null 2>&1); then
    fail "fixture $(basename "$tree") is not a consistent locked workspace"
  fi
done
pass "every tree fixture is a consistent locked workspace"

run_guard() {
  set +e
  OUT="$(bash "$GUARD" "$@" 2>&1)"
  RC=$?
  set -e
}

expect_pass() {
  local description="$1"
  shift
  run_guard "$@"
  [ "$RC" -eq 0 ] || fail "$description — expected exit 0, got rc=$RC: $OUT"
  pass "$description"
}

expect_fail_with() {
  local expected="$1" description="$2"
  shift 2
  run_guard "$@"
  [ "$RC" -ne 0 ] || fail "$description — expected a failure, got exit 0: $OUT"
  grep -Fq -- "$expected" <<<"$OUT" || fail "$description — output did not name '$expected': $OUT"
  pass "$description"
}

ok_tree="$FIXTURES/tree/ok"

# Condition A (lock files only; the tree conditions run on the GREEN tree).
expect_pass 'A: a lock with only cudarc-fathomdb passes' \
  --root "$ok_tree" --lock "$FIXTURES/lock/ok.lock"
expect_fail_with 'cudarc 0.19.4' 'upstream_cudarc_0_19_in_the_lock_is_rejected' \
  --root "$ok_tree" --lock "$FIXTURES/lock/upstream-cudarc.lock"
expect_pass 'A exclusion: ug-cuda cudarc 0.17.8 in the lock passes' \
  --root "$ok_tree" --lock "$FIXTURES/lock/ug-cudarc.lock"

# Condition C (the resolved normal tree per target and feature set).
expect_pass 'C and D: the GREEN tree fixture passes' --root "$ok_tree"
expect_fail_with 'cudarc v0.19.4' 'upstream_cudarc_in_the_tree_is_rejected' \
  --root "$FIXTURES/tree/upstream-cudarc"
expect_fail_with 'is not exactly "cudarc-fathomdb v0.19.10"' 'a_path_cudarc_fathomdb_is_rejected' \
  --root "$FIXTURES/tree/path-cudarc-fathomdb"
expect_fail_with 'no cudarc-fathomdb line for embed-cuda' 'embed_cuda_without_cudarc_fathomdb_is_rejected' \
  --root "$FIXTURES/tree/no-cudarc-fathomdb"
# The fixture's own lock must hold the optional cudarc 0.19.4 (Cargo locks
# every optional dependency), so condition A is pointed at the GREEN lock and
# this arm exercises condition C's flash-attn exclusion alone.
expect_pass 'C exclusion: cudarc reachable only through flash-attn passes' \
  --root "$FIXTURES/tree/flash-attn-only" --lock "$FIXTURES/lock/ok.lock"

# Condition D (a concrete cuda-NNNNN feature on cudarc-fathomdb, D-8W).
expect_fail_with 'cudarc-fathomdb feature "cuda-12060"' 'concrete_cuda_feature_on_cudarc_fathomdb_is_rejected' \
  --root "$FIXTURES/tree/concrete-feature"

# The real tree. --offline needs the registry cache; crates.io is a permitted
# package registry, and --locked never rewrites the lock.
(cd "$REPO_ROOT" && cargo fetch --locked >/dev/null 2>&1) || fail "cargo fetch --locked failed on the real tree"
expect_pass 'the real tree has exactly one cudarc 0.19, cudarc-fathomdb' --root "$REPO_ROOT"
