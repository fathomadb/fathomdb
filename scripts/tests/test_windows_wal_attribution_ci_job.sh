#!/usr/bin/env bash
# Slice 65: protects the hosted Windows attribution witness and its redacted
# retained artifact from a later Linux-only edit.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CI="${CI_YML:-$REPO_ROOT/.github/workflows/ci.yml}"
SOURCE_TEST="${SOURCE_TEST:-$REPO_ROOT/src/rust/crates/fathomdb-engine/tests/erasure_completeness.rs}"
ENGINE_SOURCE="${ENGINE_SOURCE:-$REPO_ROOT/src/rust/crates/fathomdb-engine/src/lib.rs}"
PY_SOURCE="$REPO_ROOT/src/rust/crates/fathomdb-py/src/lib.rs"
PY_CONTROL="$REPO_ROOT/src/python/tests/test_slice65_wal_attribution_installed.py"
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

function_body() {
  awk -v name="$2" '
    $0 ~ "^    fn " name "\\(" { inside = 1 }
    inside && $0 ~ "^    fn " && $0 !~ "^    fn " name "\\(" { exit }
    inside { print }
  ' "$1"
}

python_function_body() {
  python3 - "$1" "$2" <<'PY'
import ast
import sys

source_path, function_name = sys.argv[1:]
source = open(source_path, encoding="utf-8").read()
module = ast.parse(source, filename=source_path)
for node in module.body:
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == function_name:
        lines = source.splitlines()
        print("\n".join(lines[node.lineno - 1 : node.end_lineno]))
        break
else:
    raise SystemExit(f"top-level Python function not found: {function_name}")
PY
}

assert_before_in_function() {
  local source="$1" function="$2" first="$3" second="$4" description="$5"
  local body first_line second_line
  body="$(function_body "$source" "$function")"
  first_line="$(grep -nF -- "$first" <<<"$body" | head -n 1 | cut -d: -f1 || true)"
  second_line="$(grep -nF -- "$second" <<<"$body" | head -n 1 | cut -d: -f1 || true)"
  if [ -n "$first_line" ] && [ -n "$second_line" ] && [ "$first_line" -lt "$second_line" ]; then
    pass "$description"
  else
    fail "$description (expected $first before $second)"
  fi
}

assert_before_in_python_function() {
  local source="$1" function="$2" first="$3" second="$4" description="$5"
  local body first_line second_line
  body="$(python_function_body "$source" "$function")"
  first_line="$(grep -nF -- "$first" <<<"$body" | head -n 1 | cut -d: -f1 || true)"
  second_line="$(grep -nF -- "$second" <<<"$body" | head -n 1 | cut -d: -f1 || true)"
  if [ -n "$first_line" ] && [ -n "$second_line" ] && [ "$first_line" -lt "$second_line" ]; then
    pass "$description"
  else
    fail "$description (expected $first before $second)"
  fi
}

assert_before_in_text() {
  local text="$1" first="$2" second="$3" description="$4"
  local first_line second_line
  first_line="$(grep -nF -- "$first" <<<"$text" | head -n 1 | cut -d: -f1 || true)"
  second_line="$(grep -nF -- "$second" <<<"$text" | head -n 1 | cut -d: -f1 || true)"
  if [ -n "$first_line" ] && [ -n "$second_line" ] && [ "$first_line" -lt "$second_line" ]; then
    pass "$description"
  else
    fail "$description (expected $first before $second)"
  fi
}

JOB="$(job_block "$CI")"
CODE="$(grep -v '^[[:space:]]*#' <<<"$JOB" || true)"
if [ -n "$JOB" ]; then pass "ci.yml defines Windows WAL attribution"; else fail "ci.yml has no windows-wal-attribution job"; fi
assert_contains "$CODE" 'runs-on: windows-latest' "job runs on hosted Windows x64"
assert_contains "$CODE" 'FATHOMDB_WAL_ATTRIBUTION = "1"' "job opts into private attribution"
assert_contains "$CODE" 'wal_attribution_owned_reader_records_exact_busy_and_idle_success' "job runs retained managed-reader record contract"
assert_contains "$CODE" 'tests::wal_attribution_owned_reader_records_exact_busy_and_idle_success' "job selects the full exact lib-test path"
assert_contains "$CODE" 'wal_attribution_retained_materialized_result_is_idle_at_checkpoint' "job runs retained-result idle control"
assert_contains "$CODE" 'tests::wal_attribution_reader_handoff_is_idle_before_materialized_reply' "job selects the exact reader-handoff control"
assert_contains "$CODE" 'reader-handoff attribution command selected zero tests' "job rejects a zero-test reader-handoff invocation"
assert_contains "$CODE" 'slice65_wal reader_handoff_idle_before_reply=passed' "job requires the reader-handoff artifact marker"
assert_contains "$CODE" 'wal_attribution_reopen_recovery_reads_then_nested_erasure_are_idle' "job runs incident-shaped reopen control"
assert_contains "$CODE" 'wal_attribution_projection_worker_transaction_is_owned_then_idle' "job runs projection transaction control"
assert_contains "$CODE" 'tests::wal_attribution_post_commit_acknowledges_and_records_raw_checkpoint_diagnostic' "job selects the exact post-commit diagnostic control"
assert_contains "$CODE" 'post-commit diagnostic command selected zero tests' "job rejects a zero-test post-commit diagnostic invocation"
assert_contains "$CODE" 'slice65_wal post_commit_ack direct_inventory=' "job requires direct post-commit transaction facts"
assert_contains "$CODE" 'collector_roles=idle' "job retains collector state separately from direct transaction facts"
assert_contains "$CODE" 'slice65_wal post_commit_raw case=pre_close' "job requires redacted pre-close raw checkpoint samples"
assert_contains "$CODE" 'slice65_wal post_commit_child_raw case=after_close' "job requires the redacted fresh-child probe outcome"
assert_contains "$CODE" 'python_binding_completion_ack' "job requires installed binding completion acknowledgement"
assert_contains "$CODE" 'python_binding_direct_inventory=' "job requires direct installed binding inventory"
assert_contains "$CODE" 'python_binding_raw case=before_engine_sampler' "job requires the first installed binding raw sample"
assert_contains "$CODE" 'python_binding_engine_sampler' "job requires exactly one installed binding Engine-path sampler"
assert_contains "$CODE" 'python_binding_raw case=after_engine_sampler' "job requires the second installed binding raw sample"
assert_contains "$CODE" 'python_binding_child_raw case=after_close' "job requires the conditional installed binding child probe outcome"
assert_contains "$CODE" 'wal_attribution_close_boundary_raw_checkpoint_is_clean' "job runs direct close-boundary control"
assert_contains "$CODE" 'wal_attribution_close_boundary_fresh_open_is_clean' "job runs fresh-open close-boundary control"
assert_contains "$CODE" 'wal_attribution_close_boundary_read_get_is_clean' "job runs read-get close-boundary control"
assert_contains "$CODE" 'wal_attribution_close_boundary_neighbors_is_clean' "job runs neighbors close-boundary control"
assert_contains "$CODE" 'close-boundary direct checkpoint control selected zero tests' "job rejects zero selected direct close-boundary tests"
assert_contains "$CODE" 'close-boundary fresh-open control selected zero tests' "job rejects zero selected fresh-open close-boundary tests"
assert_contains "$CODE" 'close-boundary read-get control selected zero tests' "job rejects zero selected read-get close-boundary tests"
assert_contains "$CODE" 'close-boundary neighbors control selected zero tests' "job rejects zero selected neighbors close-boundary tests"
assert_contains "$CODE" 'test_slice65_wal_attribution_installed.py' "job runs installed-wheel controls"
assert_contains "$CODE" '--control retained' "job runs installed retained-result control"
assert_contains "$CODE" 'current_head=$(git rev-parse HEAD)' "job records current source identity"
assert_contains "$CODE" 'args: --release --out ${{ github.workspace }}/dist-slice65-test -i python3.11 --features pyo3/extension-module,test-hooks' "job builds the disposable wheel at an explicit workspace-absolute path"
assert_contains "$CODE" '$currentWheelDir = Join-Path $env:GITHUB_WORKSPACE "dist-slice65-test"' "job derives the current wheel directory from the workspace"
assert_contains "$CODE" 'Test-Path -LiteralPath $currentWheelDir -PathType Container' "job fails closed when the current wheel directory is missing"
assert_contains "$CODE" '$currentWheelCandidates = @(Get-ChildItem -LiteralPath $currentWheelDir -Filter "*.whl" -File)' "job discovers the disposable current wheel only beneath the resolved workspace directory"
assert_contains "$CODE" 'expected exactly one disposable current test-hooks wheel' "job rejects an ambiguous disposable current wheel output"
assert_contains "$CODE" 'current_wheel_path=' "job records the resolved current wheel path"
assert_contains "$CODE" 'current_wheel_sha256=' "job records current wheel digest"
assert_contains "$CODE" '& "$currentEnv\Scripts\python.exe" -m pip install --no-index --no-deps $currentWheel' "job installs exactly the resolved current wheel"
assert_absent "$CODE" 'src/python/dist-slice65-test' "job does not look for the disposable wheel beneath the Python source directory"
assert_absent "$CODE" 'Get-ChildItem -Path "dist-slice65-test"' "job has no naked relative current-wheel lookup"
assert_absent "$CODE" '-Recurse' "job does not recursively discover a current wheel"
assert_contains "$CODE" 'released_native_sha256=' "job records released native input identity"
assert_contains "$CODE" 'test-hooks' "job builds a non-shipping test-hooks wheel"
assert_contains "$CODE" 'fathomdb==0.8.22' "job installs the released 0.8.22 comparison wheel"
assert_contains "$CODE" 'serial_wheel_selector=released-0.8.22' "job retains released-wheel selector"
assert_contains "$CODE" 'serial_wheel_selector=current-source-test-hooks' "job retains current-wheel selector"
assert_contains "$CODE" '--observe-baseline-first-erase' "job explicitly observes exactly the released serial first erase"
assert_contains "$CODE" '$releasedSerialExit' "job retains the released serial process outcome"
assert_contains "$CODE" 'BASELINE_FIRST_ERASE outcome=typed_erasure_incomplete' "job parses the typed first-erase baseline observation"
assert_contains "$CODE" 'BASELINE_FIRST_ERASE outcome=clean_completion' "job parses the clean first-erase baseline observation"
assert_contains "$CODE" 'OBSERVED_EXPECTED_ERASURE_INCOMPLETE' "job emits the typed released failure observation"
assert_contains "$CODE" 'OBSERVED_CLEAN_SERIAL_COMPLETION' "job emits the clean released completion observation"
assert_contains "$CODE" '$releasedSerialExit -eq 65' "job accepts only the typed baseline observation exit"
assert_contains "$CODE" '$releasedSerialExit -eq 66' "job accepts only the clean baseline observation exit"
assert_contains "$CODE" 'released serial baseline did not produce an accepted first-erase observation' "job rejects every other released baseline process outcome"
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
  'reader_handoff_idle_before_reply=passed' \
  'reopen_nested_serial=passed' \
  'retained_materialized_idle=passed' \
  'projection_worker_transaction_ready' \
  'fn wal_attribution_post_commit_acknowledges_and_records_raw_checkpoint_diagnostic' \
  'post_commit_ack_for_test' \
  'post_commit_raw case={case}' \
  'post_commit_child_raw case=after_close' \
  'WalConnectionInventory' \
  'RuntimeConnectionInventoryRequest' \
  'report_runtime_connection_inventory_for_test' \
  'ManagedConnectionRegistry' \
  'post_commit_inventory=incomplete' \
  'fn wal_attribution_close_boundary_raw_checkpoint_is_clean' \
  'fn wal_attribution_close_boundary_fresh_open_is_clean' \
  'fn wal_attribution_close_boundary_read_get_is_clean' \
  'fn wal_attribution_close_boundary_neighbors_is_clean' \
  'unclassified_external'; do
  assert_contains "$(<"$SOURCE_TEST") $(<"$ENGINE_SOURCE")" "$marker" "source retains $marker"
done
assert_contains "$(<"$PY_SOURCE")" \
  '_arm_next_reader_snapshot_pause_for_test' \
  'wal_attribution_checkpoint_records_for_test' \
  "installed binding exposes the rendezvous only in its test-hooks build"
assert_contains "$(<"$PY_SOURCE")" \
  '_arm_next_reader_completion_pause_for_test' \
  '_wal_attribution_binding_inventory_for_test' \
  "installed binding exposes completion and direct-inventory hooks only in its test-hooks build"
assert_contains "$(<"$REPO_ROOT/src/rust/crates/fathomdb-engine/Cargo.toml")" \
  'test-hooks = []' \
  "engine reader rendezvous is a non-default test feature"
assert_contains "$(<"$ENGINE_SOURCE")" \
  '#[cfg(any(debug_assertions, feature = "test-hooks"))]' \
  "managed-reader hook is unavailable from shipped production builds"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'reader_completion_pause::fire()' \
  'binding_connection_inventory_for_test' \
  'checkpoint_at_rest_for_test' \
  "engine retains private completion, inventory, and checkpoint sampler seams"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'fresh_writer_connection_open=' \
  "source records the live fresh writer connection fact"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'ManagedConnectionCategory' \
  "source classifies every Engine-managed SQLite open"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'fn open_managed_connection' \
  "source centralizes Engine-managed SQLite opens through the audited factory"
PRODUCTION_ENGINE_SOURCE="$(sed '/^mod tests {$/,$d' "$ENGINE_SOURCE")"
FACTORY_DIRECT_OPENS="$(grep -Fc 'Connection::open(' <<<"$PRODUCTION_ENGINE_SOURCE" || true)"
if [ "$FACTORY_DIRECT_OPENS" -eq 1 ]; then
  pass "only the audited factory directly opens Engine SQLite connections"
else
  fail "Engine production source has $FACTORY_DIRECT_OPENS direct SQLite opens; expected the one audited factory"
fi
assert_contains "$(<"$PY_CONTROL")" \
  '--observe-baseline-first-erase' \
  "installed serial control has an explicit first-erase baseline observation mode"
assert_contains "$(<"$PY_CONTROL")" \
  'ErasureIncompleteError' \
  "installed serial baseline accepts only the typed erasure error"
assert_contains "$(<"$PY_CONTROL")" \
  'stage != "wal_checkpoint"' \
  "installed serial baseline rejects a non-WAL erasure stage"
assert_contains "$(<"$PY_CONTROL")" \
  'frames still in the log' \
  "installed serial baseline records normalized WAL frame evidence"
assert_contains "$(<"$PY_CONTROL")" \
  'BASELINE_FIRST_ERASE outcome=clean_completion' \
  "installed serial baseline records clean first-erase completion separately"
assert_before_in_text \
  "$(python_function_body "$PY_CONTROL" "run_serial_incident")" \
  'if not observe_baseline_first_erase:' \
  'fresh.transition' \
  "baseline observation mode does not continue to the follow-on purge lifecycle"
assert_before_in_text \
  "$CODE" \
  '--control serial' \
  '--control binding' \
  "installed serial control runs before the binding diagnostic"
assert_before_in_text \
  "$CODE" \
  '--control binding' \
  '--control retained' \
  "installed binding diagnostic runs before retained-result control"
assert_before_in_text \
  "$CODE" \
  'post-commit diagnostic command selected zero tests' \
  'serial_wheel_selector=current-source-test-hooks' \
  "current instrumented serial control runs after post-commit diagnostic artifact"
assert_before_in_python_function \
  "$PY_CONTROL" \
  "run_binding_reader_erase" \
  'python_binding_completion_ack' \
  'python_binding_direct_inventory=' \
  "binding diagnostic emits completion acknowledgement before direct inventory"
assert_before_in_python_function \
  "$PY_CONTROL" \
  "run_binding_reader_erase" \
  'python_binding_direct_inventory=' \
  '_raw_binding_checkpoint(path, "before_engine_sampler")' \
  "binding diagnostic completes direct inventory before raw samples"
assert_before_in_python_function \
  "$PY_CONTROL" \
  "run_binding_reader_erase" \
  '_raw_binding_checkpoint(path, "before_engine_sampler")' \
  'samples = native._checkpoint_at_rest_for_test()' \
  "binding diagnostic places the Engine sampler after raw sample one"
assert_before_in_python_function \
  "$PY_CONTROL" \
  "run_binding_reader_erase" \
  'samples = native._checkpoint_at_rest_for_test()' \
  '_raw_binding_checkpoint(path, "after_engine_sampler")' \
  "binding diagnostic places raw sample two after the Engine sampler"
binding_erase_calls="$(python_function_body "$PY_CONTROL" "run_binding_reader_erase" | grep -Fc 'engine.erase_source(' || true)"
if [ "$binding_erase_calls" -eq 1 ]; then
  pass "binding diagnostic retains exactly one original erasure"
else
  fail "binding diagnostic must retain exactly one original erasure; found $binding_erase_calls"
fi
for control in \
  wal_attribution_close_boundary_fresh_open_is_clean \
  wal_attribution_close_boundary_read_get_is_clean \
  wal_attribution_close_boundary_neighbors_is_clean; do
  assert_before_in_function \
    "$ENGINE_SOURCE" \
    "$control" \
    'raw_close_boundary_checkpoint' \
    'fresh.engine.close().expect("fresh close")' \
    "$control probes while the fresh Engine remains open"
done

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

  HANDOFF_SELECTOR_MUTATED="$TMPROOT/ci-without-reader-handoff-selector.yml"
  sed '/tests::wal_attribution_reader_handoff_is_idle_before_materialized_reply/d' "$CI" \
    >"$HANDOFF_SELECTOR_MUTATED"
  set +e
  handoff_selector_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$HANDOFF_SELECTOR_MUTATED" bash "$0" 2>&1)"
  handoff_selector_rc=$?
  set -e
  if [ "$handoff_selector_rc" -ne 0 ] \
    && grep -Fq 'job selects the exact reader-handoff control (missing: tests::wal_attribution_reader_handoff_is_idle_before_materialized_reply)' <<<"$handoff_selector_out"; then
    pass "mutation proves reader-handoff selector assertion is load-bearing"
  else
    fail "mutation did not fail reader-handoff selector assertion: $handoff_selector_out"
  fi

  HANDOFF_GUARD_MUTATED="$TMPROOT/ci-without-reader-handoff-guard.yml"
  sed '/reader-handoff attribution command selected zero tests/d' "$CI" >"$HANDOFF_GUARD_MUTATED"
  set +e
  handoff_guard_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$HANDOFF_GUARD_MUTATED" bash "$0" 2>&1)"
  handoff_guard_rc=$?
  set -e
  if [ "$handoff_guard_rc" -ne 0 ] \
    && grep -Fq 'job rejects a zero-test reader-handoff invocation (missing: reader-handoff attribution command selected zero tests)' <<<"$handoff_guard_out"; then
    pass "mutation proves reader-handoff zero-test guard assertion is load-bearing"
  else
    fail "mutation did not fail reader-handoff zero-test guard assertion: $handoff_guard_out"
  fi

  HANDOFF_MARKER_MUTATED="$TMPROOT/ci-without-reader-handoff-marker.yml"
  sed '/slice65_wal reader_handoff_idle_before_reply=passed/d' "$CI" >"$HANDOFF_MARKER_MUTATED"
  set +e
  handoff_marker_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$HANDOFF_MARKER_MUTATED" bash "$0" 2>&1)"
  handoff_marker_rc=$?
  set -e
  if [ "$handoff_marker_rc" -ne 0 ] \
    && grep -Fq 'job requires the reader-handoff artifact marker (missing: slice65_wal reader_handoff_idle_before_reply=passed)' <<<"$handoff_marker_out"; then
    pass "mutation proves reader-handoff artifact assertion is load-bearing"
  else
    fail "mutation did not fail reader-handoff artifact assertion: $handoff_marker_out"
  fi

  POST_COMMIT_SELECTOR_MUTATED="$TMPROOT/ci-without-post-commit-selector.yml"
  sed '/tests::wal_attribution_post_commit_acknowledges_and_records_raw_checkpoint_diagnostic/d' "$CI" \
    >"$POST_COMMIT_SELECTOR_MUTATED"
  set +e
  post_commit_selector_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$POST_COMMIT_SELECTOR_MUTATED" bash "$0" 2>&1)"
  post_commit_selector_rc=$?
  set -e
  if [ "$post_commit_selector_rc" -ne 0 ] \
    && grep -Fq 'job selects the exact post-commit diagnostic control (missing: tests::wal_attribution_post_commit_acknowledges_and_records_raw_checkpoint_diagnostic)' <<<"$post_commit_selector_out"; then
    pass "mutation proves post-commit selector assertion is load-bearing"
  else
    fail "mutation did not fail post-commit selector assertion: $post_commit_selector_out"
  fi

  POST_COMMIT_GUARD_MUTATED="$TMPROOT/ci-without-post-commit-guard.yml"
  sed '/post-commit diagnostic command selected zero tests/d' "$CI" >"$POST_COMMIT_GUARD_MUTATED"
  set +e
  post_commit_guard_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$POST_COMMIT_GUARD_MUTATED" bash "$0" 2>&1)"
  post_commit_guard_rc=$?
  set -e
  if [ "$post_commit_guard_rc" -ne 0 ] \
    && grep -Fq 'job rejects a zero-test post-commit diagnostic invocation (missing: post-commit diagnostic command selected zero tests)' <<<"$post_commit_guard_out"; then
    pass "mutation proves post-commit zero-test guard assertion is load-bearing"
  else
    fail "mutation did not fail post-commit zero-test guard assertion: $post_commit_guard_out"
  fi

  POST_COMMIT_ARTIFACT_MUTATED="$TMPROOT/ci-without-post-commit-artifact.yml"
  sed '/slice65_wal post_commit_ack direct_inventory=/d' "$CI" >"$POST_COMMIT_ARTIFACT_MUTATED"
  set +e
  post_commit_artifact_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$POST_COMMIT_ARTIFACT_MUTATED" bash "$0" 2>&1)"
  post_commit_artifact_rc=$?
  set -e
  if [ "$post_commit_artifact_rc" -ne 0 ] \
    && grep -Fq 'job requires direct post-commit transaction facts (missing: slice65_wal post_commit_ack direct_inventory=)' <<<"$post_commit_artifact_out"; then
    pass "mutation proves post-commit artifact assertion is load-bearing"
  else
    fail "mutation did not fail post-commit artifact assertion: $post_commit_artifact_out"
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

  WHEEL_PATH_MUTATED="$TMPROOT/ci-without-wheel-path.yml"
  sed '/current_wheel_path=/d' "$CI" >"$WHEEL_PATH_MUTATED"
  set +e
  wheel_path_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$WHEEL_PATH_MUTATED" bash "$0" 2>&1)"
  wheel_path_rc=$?
  set -e
  if [ "$wheel_path_rc" -ne 0 ] \
    && grep -Fq 'job records the resolved current wheel path (missing: current_wheel_path=)' <<<"$wheel_path_out"; then
    pass "mutation proves current-wheel resolved-path assertion is load-bearing"
  else
    fail "mutation did not fail current-wheel resolved-path assertion: $wheel_path_out"
  fi

  WHEEL_BUILD_OUT_MUTATED="$TMPROOT/ci-with-relative-wheel-out.yml"
  sed 's#${{ github.workspace }}/dist-slice65-test#dist-slice65-test#' "$CI" >"$WHEEL_BUILD_OUT_MUTATED"
  set +e
  wheel_build_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$WHEEL_BUILD_OUT_MUTATED" bash "$0" 2>&1)"
  wheel_build_out_rc=$?
  set -e
  if [ "$wheel_build_out_rc" -ne 0 ] \
    && grep -Fq 'job builds the disposable wheel at an explicit workspace-absolute path (missing: args: --release --out ${{ github.workspace }}/dist-slice65-test -i python3.11 --features pyo3/extension-module,test-hooks)' <<<"$wheel_build_out"; then
    pass "mutation proves workspace-absolute wheel build path is load-bearing"
  else
    fail "mutation did not fail workspace-absolute wheel build path assertion: $wheel_build_out"
  fi

  WHEEL_DIRECTORY_MUTATED="$TMPROOT/ci-without-wheel-directory.yml"
  sed '/\$currentWheelDir = Join-Path \$env:GITHUB_WORKSPACE "dist-slice65-test"/d' "$CI" >"$WHEEL_DIRECTORY_MUTATED"
  set +e
  wheel_directory_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$WHEEL_DIRECTORY_MUTATED" bash "$0" 2>&1)"
  wheel_directory_rc=$?
  set -e
  if [ "$wheel_directory_rc" -ne 0 ] \
    && grep -Fq 'job derives the current wheel directory from the workspace (missing: $currentWheelDir = Join-Path $env:GITHUB_WORKSPACE "dist-slice65-test")' <<<"$wheel_directory_out"; then
    pass "mutation proves workspace-derived wheel directory is load-bearing"
  else
    fail "mutation did not fail workspace-derived wheel directory assertion: $wheel_directory_out"
  fi

  WHEEL_LITERAL_LOOKUP_MUTATED="$TMPROOT/ci-without-wheel-literal-lookup.yml"
  sed 's/Get-ChildItem -LiteralPath \$currentWheelDir/Get-ChildItem -Path \$currentWheelDir/' "$CI" >"$WHEEL_LITERAL_LOOKUP_MUTATED"
  set +e
  wheel_literal_lookup_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$WHEEL_LITERAL_LOOKUP_MUTATED" bash "$0" 2>&1)"
  wheel_literal_lookup_rc=$?
  set -e
  if [ "$wheel_literal_lookup_rc" -ne 0 ] \
    && grep -Fq 'job discovers the disposable current wheel only beneath the resolved workspace directory (missing: $currentWheelCandidates = @(Get-ChildItem -LiteralPath $currentWheelDir -Filter "*.whl" -File))' <<<"$wheel_literal_lookup_out"; then
    pass "mutation proves literal current-wheel lookup is load-bearing"
  else
    fail "mutation did not fail literal current-wheel lookup assertion: $wheel_literal_lookup_out"
  fi

  BASELINE_TYPED_OBSERVATION_MUTATED="$TMPROOT/ci-without-typed-baseline-observation.yml"
  sed '/OBSERVED_EXPECTED_ERASURE_INCOMPLETE/d' "$CI" >"$BASELINE_TYPED_OBSERVATION_MUTATED"
  set +e
  baseline_typed_observation_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$BASELINE_TYPED_OBSERVATION_MUTATED" bash "$0" 2>&1)"
  baseline_typed_observation_rc=$?
  set -e
  if [ "$baseline_typed_observation_rc" -ne 0 ] \
    && grep -Fq 'job emits the typed released failure observation (missing: OBSERVED_EXPECTED_ERASURE_INCOMPLETE)' <<<"$baseline_typed_observation_out"; then
    pass "mutation proves typed released baseline observation is load-bearing"
  else
    fail "mutation did not fail typed released baseline observation assertion: $baseline_typed_observation_out"
  fi

  BASELINE_CLEAN_OBSERVATION_MUTATED="$TMPROOT/ci-without-clean-baseline-observation.yml"
  sed '/OBSERVED_CLEAN_SERIAL_COMPLETION/d' "$CI" >"$BASELINE_CLEAN_OBSERVATION_MUTATED"
  set +e
  baseline_clean_observation_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$BASELINE_CLEAN_OBSERVATION_MUTATED" bash "$0" 2>&1)"
  baseline_clean_observation_rc=$?
  set -e
  if [ "$baseline_clean_observation_rc" -ne 0 ] \
    && grep -Fq 'job emits the clean released completion observation (missing: OBSERVED_CLEAN_SERIAL_COMPLETION)' <<<"$baseline_clean_observation_out"; then
    pass "mutation proves clean released baseline observation is load-bearing"
  else
    fail "mutation did not fail clean released baseline observation assertion: $baseline_clean_observation_out"
  fi

  BASELINE_CLEAN_EXIT_MUTATED="$TMPROOT/ci-without-clean-baseline-exit.yml"
  sed '/\$releasedSerialExit -eq 66/d' "$CI" >"$BASELINE_CLEAN_EXIT_MUTATED"
  set +e
  baseline_clean_exit_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$BASELINE_CLEAN_EXIT_MUTATED" bash "$0" 2>&1)"
  baseline_clean_exit_rc=$?
  set -e
  if [ "$baseline_clean_exit_rc" -ne 0 ] \
    && grep -Fq 'job accepts only the clean baseline observation exit (missing: $releasedSerialExit -eq 66)' <<<"$baseline_clean_exit_out"; then
    pass "mutation proves clean baseline exit pairing is load-bearing"
  else
    fail "mutation did not fail clean baseline exit-pair assertion: $baseline_clean_exit_out"
  fi

  FRESH_CONNECTION_MUTATED="$TMPROOT/lib-without-fresh-connection-marker.rs"
  sed '/fresh_writer_connection_open=/d' "$ENGINE_SOURCE" >"$FRESH_CONNECTION_MUTATED"
  set +e
  fresh_connection_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$FRESH_CONNECTION_MUTATED" bash "$0" 2>&1)"
  fresh_connection_rc=$?
  set -e
  if [ "$fresh_connection_rc" -ne 0 ] \
    && grep -Fq 'source records the live fresh writer connection fact (missing: fresh_writer_connection_open=)' <<<"$fresh_connection_out"; then
    pass "mutation proves live fresh writer fact is load-bearing"
  else
    fail "mutation did not fail live fresh writer fact assertion: $fresh_connection_out"
  fi

  BINDING_ORDER_MUTATED="$TMPROOT/ci-without-binding-command.yml"
  sed '/--control binding --wheel-version/d' "$CI" >"$BINDING_ORDER_MUTATED"
  set +e
  binding_order_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$BINDING_ORDER_MUTATED" bash "$0" 2>&1)"
  binding_order_rc=$?
  set -e
  if [ "$binding_order_rc" -ne 0 ] \
    && grep -Fq 'installed serial control runs before the binding diagnostic (expected --control serial before --control binding)' <<<"$binding_order_out"; then
    pass "mutation proves installed serial-to-binding ordering is load-bearing"
  else
    fail "mutation did not fail installed serial-to-binding ordering assertion: $binding_order_out"
  fi

  while IFS='|' read -r selector guard; do
    CLOSE_BOUNDARY_GUARD_MUTATED="$TMPROOT/ci-without-${selector}.yml"
    sed "/${guard}/d" "$CI" >"$CLOSE_BOUNDARY_GUARD_MUTATED"
    set +e
    close_boundary_guard_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$CLOSE_BOUNDARY_GUARD_MUTATED" bash "$0" 2>&1)"
    close_boundary_guard_rc=$?
    set -e
    if [ "$close_boundary_guard_rc" -ne 0 ] \
      && grep -Fq "job rejects zero selected ${selector} close-boundary tests" <<<"$close_boundary_guard_out"; then
      pass "mutation proves ${selector} close-boundary zero-test guard is load-bearing"
    else
      fail "mutation did not fail ${selector} close-boundary zero-test guard: $close_boundary_guard_out"
    fi
  done <<'EOF'
direct|close-boundary direct checkpoint control selected zero tests
fresh-open|close-boundary fresh-open control selected zero tests
read-get|close-boundary read-get control selected zero tests
neighbors|close-boundary neighbors control selected zero tests
EOF
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
