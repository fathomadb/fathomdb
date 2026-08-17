#!/usr/bin/env bash
# Slice 65: protects the hosted Windows attribution witness and its redacted
# retained artifact from a later Linux-only edit.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CI="${CI_YML:-$REPO_ROOT/.github/workflows/ci.yml}"
SOURCE_TEST="${SOURCE_TEST:-$REPO_ROOT/src/rust/crates/fathomdb-engine/tests/erasure_completeness.rs}"
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
assert_contains "$CODE" 'erasure_busy_cross_process_windows_yields_typed_diagnostic' "job retains external-reader comparison"
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
  'owned_reader_snapshot' \
  'unclassified_external'; do
  assert_contains "$(<"$SOURCE_TEST") $(<"$REPO_ROOT/src/rust/crates/fathomdb-engine/src/lib.rs")" "$marker" "source retains $marker"
done
assert_absent "$(<"$REPO_ROOT/src/rust/crates/fathomdb-engine/src/lib.rs")" \
  'pub fn pause_reader_after_wal_snapshot_for_test' \
  "managed-reader hook is not published"

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
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
