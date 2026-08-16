#!/usr/bin/env bash
# Slice 60: regression coverage for the first-party Windows WAL diagnosis job.
# The source test is Windows-only, so this suite protects the CI wiring and
# retained diagnostic artifact from silently disappearing on a Linux edit.
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
    /^  windows-wal-checkpoint-diagnosis:/ { inblock = 1; print; next }
    inblock && /^  [A-Za-z0-9_-]+:/ { inblock = 0 }
    inblock { print }
  ' "$1"
}

assert_contains() {
  local body="$1" needle="$2" description="$3"
  if grep -Fq -- "$needle" <<<"$body"; then
    pass "$description"
  else
    fail "$description (missing: $needle)"
  fi
}

assert_source_marker() {
  local marker="$1" description="$2"
  if grep -Fq -- "$marker" "$SOURCE_TEST"; then
    pass "$description"
  else
    fail "$description (missing source marker: $marker)"
  fi
}

JOB="$(job_block "$CI")"
CODE="$(grep -v '^[[:space:]]*#' <<<"$JOB" || true)"

if [ -n "$JOB" ]; then
  pass "ci.yml defines the Windows WAL checkpoint diagnosis job"
else
  fail "ci.yml has no windows-wal-checkpoint-diagnosis job"
fi

if [ -n "$JOB" ]; then
  assert_contains "$CODE" 'runs-on: windows-latest' "job runs on first-party Windows x64"
  assert_contains "$CODE" 'needs: changes' "job follows the non-docs changes boundary"
  assert_contains "$CODE" "needs.changes.outputs.docs_only != 'true'" "job does not run for markdown-only changes"
  assert_contains "$CODE" 'cargo test -p fathomdb-engine --features operator --test erasure_completeness' "job invokes the engine erasure test binary"
  assert_contains "$CODE" 'erasure_busy_cross_process_windows_yields_typed_diagnostic' "job selects the cross-process Windows diagnostic"
  assert_contains "$CODE" '--nocapture' "job retains test-emitted lock/timing diagnostics"
  assert_contains "$CODE" 'Tee-Object -FilePath "$env:RUNNER_TEMP\slice60-windows-wal-diagnostic.log"' "job writes diagnostics to an artifact file"
  assert_contains "$CODE" 'if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }' "job preserves cargo test failures"
  assert_contains "$CODE" 'if: always()' "job uploads diagnostics after a failure"
  assert_contains "$CODE" 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a' "job pins the diagnostic uploader"
  assert_contains "$CODE" '${{ runner.temp }}/slice60-windows-wal-diagnostic.log' "upload path matches the diagnostic file"
fi

assert_source_marker 'slice60_windows_wal child_started pid=' "child start/PID is retained"
assert_source_marker 'slice60_windows_wal child_ready pid=' "child ready sentinel is retained"
assert_source_marker 'slice60_windows_wal parent_ready_observed child_pid=' "parent ready observation is retained"
assert_source_marker 'slice60_windows_wal checkpoint_start child_pid=' "checkpoint start is retained"
assert_source_marker 'slice60_windows_wal checkpoint_result elapsed_ms=' "checkpoint result and elapsed time are retained"
assert_source_marker 'slice60_windows_wal release_signaled child_pid=' "parent release signal is retained"
assert_source_marker 'slice60_windows_wal child_release_observed pid=' "child release observation is retained"
assert_source_marker 'slice60_windows_wal child_reaped child_pid=' "successful child reaping is retained"

if [ "${WINDOWS_WAL_CI_FIXTURE:-0}" != "1" ]; then
  TMPROOT="$(mktemp -d)"
  cleanup() {
    case "$TMPROOT" in
      "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$TMPROOT" ;;
      *) printf 'refusing to remove unexpected temp path: %s\n' "$TMPROOT" >&2 ;;
    esac
  }
  trap cleanup EXIT

  MUTATED="$TMPROOT/ci.yml"
  sed '/^  windows-wal-checkpoint-diagnosis:/,/^  markdownlint:/d' "$CI" >"$MUTATED"
  set +e
  mutation_out="$(WINDOWS_WAL_CI_FIXTURE=1 CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] \
    && grep -Fq 'ci.yml has no windows-wal-checkpoint-diagnosis job' <<<"$mutation_out"; then
    pass "mutation proves the job-existence assertion is load-bearing"
  else
    fail "mutation did not make the job-existence assertion fail: $mutation_out"
  fi

  MUTATED_SOURCE="$TMPROOT/erasure_completeness.rs"
  sed '/slice60_windows_wal child_started pid=/d' "$SOURCE_TEST" >"$MUTATED_SOURCE"
  set +e
  marker_mutation_out="$(WINDOWS_WAL_CI_FIXTURE=1 SOURCE_TEST="$MUTATED_SOURCE" bash "$0" 2>&1)"
  marker_mutation_rc=$?
  set -e
  if [ "$marker_mutation_rc" -ne 0 ] \
    && grep -Fq 'child start/PID is retained (missing source marker' <<<"$marker_mutation_out"; then
    pass "marker mutation proves retained child-start evidence is load-bearing"
  else
    fail "marker mutation did not make the source-marker assertion fail: $marker_mutation_out"
  fi

  if actionlint "$CI"; then
    pass "actionlint accepts the Windows WAL diagnosis workflow"
  else
    fail "actionlint rejects the Windows WAL diagnosis workflow"
  fi
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
