#!/usr/bin/env bash
# Slice 65: protects the hosted Windows attribution witness and its redacted
# retained artifact from a later Linux-only edit.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CI="${CI_YML:-$REPO_ROOT/.github/workflows/ci.yml}"
SOURCE_TEST="${SOURCE_TEST:-$REPO_ROOT/src/rust/crates/fathomdb-engine/tests/erasure_completeness.rs}"
PY_SOURCE="$REPO_ROOT/src/rust/crates/fathomdb-py/src/lib.rs"
PASSED=0
FAILED=0

pass() { printf 'PASS  %s\n' "$1"; PASSED=$((PASSED + 1)); }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

job_block() {
  awk '
    /^  windows-wal-attribution:/ { inblock = 1; print; next }
    inblock && /^  [A-Za-z0-9_-]+:/ { inblock = 0 }
    inblock { print }
  ' "$1"
}

assert_contains() {
  if grep -Fq -- "$2" <<<"$1"; then pass "$3"; else fail "$3 (missing: $2)"; fi
}

assert_absent() {
  if grep -Fq -- "$2" <<<"$1"; then fail "$3 (unexpected: $2)"; else pass "$3"; fi
}

JOB="$(job_block "$CI")"
CODE="$(grep -v '^[[:space:]]*#' <<<"$JOB" || true)"
if [ -n "$JOB" ]; then pass "ci.yml defines Windows WAL attribution"; else fail "ci.yml has no windows-wal-attribution job"; fi
assert_contains "$CODE" 'runs-on: windows-latest' "job runs on hosted Windows x64"
assert_contains "$CODE" 'FATHOMDB_WAL_ATTRIBUTION = "1"' "job opts into private attribution"
assert_contains "$CODE" 'wal_attribution_owned_reader_records_exact_busy_and_idle_success' "job runs retained managed-reader record contract"
assert_contains "$CODE" 'tests::wal_attribution_owned_reader_records_exact_busy_and_idle_success' "job selects the full exact lib-test path"
assert_contains "$CODE" 'wal_attribution_retained_materialized_result_is_idle_at_checkpoint' "job runs retained-result idle control"
assert_contains "$CODE" 'wal_attribution_reopen_recovery_reads_then_nested_erasure_are_idle' "job runs incident-shaped reopen control"
assert_contains "$CODE" 'wal_attribution_projection_worker_transaction_is_owned_then_idle' "job runs projection transaction control"
assert_contains "$CODE" 'test_slice65_wal_attribution_installed.py' "job runs installed-wheel controls"
assert_contains "$CODE" '--control retained' "job runs installed retained-result control"
assert_contains "$CODE" 'current_head=$(git rev-parse HEAD)' "job records current source identity"
assert_contains "$CODE" 'current_wheel_sha256=' "job records current wheel digest"
assert_contains "$CODE" 'released_native_sha256=' "job records released native input identity"
assert_contains "$CODE" 'test-hooks' "job builds a non-shipping test-hooks wheel"
assert_contains "$CODE" 'fathomdb==0.8.22' "job installs the released 0.8.22 comparison wheel"
assert_contains "$CODE" 'serial_wheel_selector=released-0.8.22' "job retains released-wheel selector"
assert_contains "$CODE" 'serial_wheel_selector=current-source-test-hooks' "job retains current-wheel selector"
assert_contains "$CODE" 'serial_current_attribution_expected=1' "job validates current serial attribution marker"
assert_contains "$CODE" 'serial_idle_after_read_get=passed' "job retains immediate serial read.get idle marker"
assert_contains "$CODE" 'serial_idle_after_neighbors=passed' "job retains immediate serial neighbors idle marker"
assert_contains "$CODE" 'erasure_busy_cross_process_windows_yields_typed_diagnostic' "job retains external-reader comparison"
assert_contains "$CODE" 'external-reader attribution command selected zero tests' "job rejects a zero-test external-reader invocation"
assert_contains "$CODE" 'running 1 test' "job rejects a zero-test managed-reader invocation"
assert_contains "$CODE" 'Select-String' "job checks retained output for an executed test"
assert_contains "$CODE" '--nocapture' "job retains diagnostic lifecycle markers"
assert_contains "$CODE" 'slice65-windows-wal-attribution.log' "job writes Slice 65 artifact"
assert_contains "$CODE" 'slice65-windows-wal-attribution-${{ github.run_id }}-${{ github.run_attempt }}' "artifact name is unique"
assert_contains "$CODE" 'if: always()' "artifact uploads after a failure"
assert_contains "$CODE" 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a' "artifact uploader is pinned"

for marker in \
  'slice65_wal managed_reader_snapshot_ready' \
  'slice65_wal managed_reader_snapshot_released' \
  'pause_reader_after_wal_snapshot_for_test' \
  'wal_attribution_checkpoints_for_test' \
  'retained_record_contract=passed' \
  'active_roles == vec![(WalAttributionRole::ReaderWorker, 0)]' \
  'SELECT COUNT(*) FROM canonical_nodes' \
  'owned_reader_snapshot' \
  'owned_runtime_transaction' \
  'reopen_nested_serial=passed' \
  'retained_materialized_idle=passed' \
  'projection_worker_transaction_ready' \
  'unclassified_external'; do
  assert_contains "$(<"$SOURCE_TEST") $(<"$REPO_ROOT/src/rust/crates/fathomdb-engine/src/lib.rs")" "$marker" "source retains $marker"
done
assert_contains "$(<"$PY_SOURCE")" \
  '_arm_next_reader_snapshot_pause_for_test' \
  'wal_attribution_checkpoint_records_for_test' \
  "installed binding exposes the rendezvous only in its test-hooks build"
assert_contains "$(<"$REPO_ROOT/src/rust/crates/fathomdb-engine/Cargo.toml")" \
  'test-hooks = []' \
  "engine reader rendezvous is a non-default test feature"
assert_contains "$(<"$REPO_ROOT/src/rust/crates/fathomdb-engine/src/lib.rs")" \
  '#[cfg(any(debug_assertions, feature = "test-hooks"))]' \
  "managed-reader hook is unavailable from shipped production builds"

if [ "${WINDOWS_WAL_ATTRIBUTION_FIXTURE:-0}" != "1" ]; then
  TMPROOT="$(mktemp -d)"
  trap 'rm -rf "$TMPROOT"' EXIT
  MUTATED="$TMPROOT/ci.yml"
  sed '/^  windows-wal-attribution:/,/^  wheel-size-gate:/d' "$CI" >"$MUTATED"
  set +e
  out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$MUTATED" bash "$0" 2>&1)"
  rc=$?
  set -e
  if [ "$rc" -ne 0 ] && grep -Fq 'ci.yml has no windows-wal-attribution job' <<<"$out"; then
    pass "mutation proves attribution job assertion is load-bearing"
  else
    fail "mutation did not fail attribution job assertion: $out"
  fi

  EXECUTION_GUARD_MUTATED="$TMPROOT/ci-without-execution-guard.yml"
  sed '/Select-String -Path/d;/managed-reader attribution command selected zero tests/d' "$CI" \
    >"$EXECUTION_GUARD_MUTATED"
  set +e
  execution_guard_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$EXECUTION_GUARD_MUTATED" bash "$0" 2>&1)"
  execution_guard_rc=$?
  set -e
  if [ "$execution_guard_rc" -ne 0 ] \
    && grep -Fq 'job checks retained output for an executed test (missing: Select-String)' <<<"$execution_guard_out"; then
    pass "mutation proves zero-test execution guard is load-bearing"
  else
    fail "mutation did not fail zero-test execution guard: $execution_guard_out"
  fi

  IDENTITY_MUTATED="$TMPROOT/ci-without-wheel-identity.yml"
  sed '/current_wheel_sha256=/d' "$CI" >"$IDENTITY_MUTATED"
  set +e
  identity_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$IDENTITY_MUTATED" bash "$0" 2>&1)"
  identity_rc=$?
  set -e
  if [ "$identity_rc" -ne 0 ] \
    && grep -Fq 'job records current wheel digest (missing: current_wheel_sha256=)' <<<"$identity_out"; then
    pass "mutation proves current-wheel identity assertion is load-bearing"
  else
    fail "mutation did not fail current-wheel identity assertion: $identity_out"
  fi
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
