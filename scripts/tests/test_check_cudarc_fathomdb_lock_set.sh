#!/usr/bin/env bash
# 0.8.28 Slice 31 (AC31-14, design § 7.2): the cudarc-fathomdb lock-set guard.
# Hermetic arms use hand-minimal `cargo metadata` JSON (one object per Linux
# target) and a minimal Cargo.lock for checksums (--lock), so they need
# neither cargo nor a consistent workspace. The last arm is the real tree
# against the record copied from the cudarc fork.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
GUARD="$REPO_ROOT/scripts/check-cudarc-fathomdb-lock-set.py"
FIXTURES="$REPO_ROOT/scripts/tests/fixtures/cudarc-lock-set"

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

[ -f "$GUARD" ] || fail "guard scripts/check-cudarc-fathomdb-lock-set.py is absent"

run_fixture() {
  local name="$1"
  set +e
  OUT="$(python3 "$GUARD" --metadata "$FIXTURES/$name/metadata.json" \
    --lock "$FIXTURES/$name/Cargo.lock" --record "$FIXTURES/$name/record.json" 2>&1)"
  RC=$?
  set -e
}

expect_pass() {
  local name="$1" description="$2"
  run_fixture "$name"
  [ "$RC" -eq 0 ] || fail "$description — expected exit 0, got rc=$RC: $OUT"
  pass "$description"
}

expect_fail_with() {
  local name="$1" expected="$2" description="$3"
  run_fixture "$name"
  [ "$RC" -eq 1 ] || fail "$description — expected exit 1, got rc=$RC: $OUT"
  grep -Fq -- "$expected" <<<"$OUT" || fail "$description — output did not name '$expected': $OUT"
  pass "$description"
}

expect_pass matching 'matching_metadata_passes'
expect_fail_with changed-libloading 'changed: libloading 0.9.0 -> 0.9.1' \
  'changed_libloading_without_a_record_fails'
expect_fail_with added-dependency 'added: libm 0.2.15' 'added_dependency_fails'
expect_fail_with removed-dependency 'removed: float8 0.4.2' 'removed_dependency_fails'
expect_fail_with changed-root-features 'root features removed: f8' 'changed_root_features_fail'
expect_pass build-only-change 'build_only_dependency_change_passes'
expect_pass windows-only-change 'windows_only_dependency_change_passes'
expect_fail_with wrong-set-sha256 'record set_sha256 does not match its own contents' \
  'record_with_a_wrong_set_sha256_fails'
expect_pass unrelated-lock-change 'unrelated_lock_change_passes'

# A drift failure tells the operator how to refresh the record.
run_fixture changed-libloading
grep -Fq 'raw.githubusercontent.com' <<<"$OUT" || fail "a drift failure must give the refresh instruction: $OUT"
pass 'a drift failure gives the fork refresh instruction'

# The real tree: `cargo metadata --offline --locked` per Linux target against
# the record copied from the fork's hosted-test run with this Cargo.lock.
(cd "$REPO_ROOT" && cargo fetch --locked >/dev/null 2>&1) || fail "cargo fetch --locked failed on the real tree"
set +e
OUT="$(python3 "$GUARD" --root "$REPO_ROOT" --record "$REPO_ROOT/scripts/cudarc-fathomdb-lock-set.json" 2>&1)"
RC=$?
set -e
[ "$RC" -eq 0 ] || fail "the real tree must match the fork's recorded lock set: $OUT"
pass 'the real tree matches the fork record'
