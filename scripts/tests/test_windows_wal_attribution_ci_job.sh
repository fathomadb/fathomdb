#!/usr/bin/env bash
# Slice 65: protects the hosted Windows attribution witness and its redacted
# retained artifact from a later Linux-only edit.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CI="${CI_YML:-$REPO_ROOT/.github/workflows/ci.yml}"
SOURCE_TEST="${SOURCE_TEST:-$REPO_ROOT/src/rust/crates/fathomdb-engine/tests/erasure_completeness.rs}"
ENGINE_SOURCE="${ENGINE_SOURCE:-$REPO_ROOT/src/rust/crates/fathomdb-engine/src/lib.rs}"
PY_SOURCE="${PY_SOURCE:-$REPO_ROOT/src/rust/crates/fathomdb-py/src/lib.rs}"
PY_CONTROL="${PY_CONTROL:-$REPO_ROOT/src/python/tests/test_slice65_wal_attribution_installed.py}"
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
  first_line="$(grep -nFm1 -- "$first" <<<"$body" | cut -d: -f1 || true)"
  second_line="$(grep -nFm1 -- "$second" <<<"$body" | cut -d: -f1 || true)"
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
  first_line="$(grep -nFm1 -- "$first" <<<"$body" | cut -d: -f1 || true)"
  second_line="$(grep -nFm1 -- "$second" <<<"$body" | cut -d: -f1 || true)"
  if [ -n "$first_line" ] && [ -n "$second_line" ] && [ "$first_line" -lt "$second_line" ]; then
    pass "$description"
  else
    fail "$description (expected $first before $second)"
  fi
}

assert_before_in_text() {
  local text="$1" first="$2" second="$3" description="$4"
  local first_line second_line
  first_line="$(grep -nFm1 -- "$first" <<<"$text" | cut -d: -f1 || true)"
  second_line="$(grep -nFm1 -- "$second" <<<"$text" | cut -d: -f1 || true)"
  if [ -n "$first_line" ] && [ -n "$second_line" ] && [ "$first_line" -lt "$second_line" ]; then
    pass "$description"
  else
    fail "$description (expected $first before $second)"
  fi
}

assert_cfg_test_near_marker() {
  local source="$1" marker="$2" description="$3"
  if awk -v marker="$marker" '
    /#\[cfg\(test\)\]/{ last_test_cfg = NR }
    index($0, marker) { exit !(NR - last_test_cfg >= 0 && NR - last_test_cfg <= 5) }
    END { if (!found) {} }
  ' "$source"; then
    pass "$description"
  else
    fail "$description (missing nearby #[cfg(test)] for: $marker)"
  fi
}

JOB="$(job_block "$CI")"
CODE="$(grep -v '^[[:space:]]*#' <<<"$JOB" || true)"
if [ -n "$JOB" ]; then pass "ci.yml defines Windows WAL attribution"; else fail "ci.yml has no windows-wal-attribution job"; fi
assert_contains "$CODE" 'runs-on: windows-latest' "job runs on hosted Windows x64"
assert_contains "$CODE" 'FATHOMDB_WAL_ATTRIBUTION = "1"' "job opts into private attribution"
assert_contains "$CODE" 'wal_attribution_owned_reader_typed_refusal_then_post_release_sampler_is_recorded' "job runs truthful managed-reader record contract"
assert_contains "$CODE" 'tests::wal_attribution_owned_reader_typed_refusal_then_post_release_sampler_is_recorded' "job selects the truthful full exact lib-test path"
assert_contains "$CODE" 'managed-reader attribution command selected zero tests' "job rejects a zero-test managed-reader invocation"
assert_contains "$CODE" 'managed_reader_original_erase=typed_erasure_incomplete owned_busy_attempts=5' "job requires the original managed-reader typed busy marker"
assert_contains "$CODE" 'managed_reader_completion_ack reader_autocommit=1 collector_roles=idle' "job requires post-finish managed-reader acknowledgement"
assert_contains "$CODE" 'managed_reader_native_state_inventory=state_inventory=complete reason=complete' "job requires complete managed-reader native-state inventory"
assert_contains "$CODE" 'managed_reader_sampler_native_state control=binding_sampler phase=before' "job requires managed-reader sampler before facts"
assert_contains "$CODE" 'managed_reader_sampler_native_state control=binding_sampler phase=after' "job requires managed-reader sampler after facts"
assert_contains "$CODE" 'managed_reader_sampler_terminal outcome=' "job requires managed-reader sampler terminal outcome"
assert_contains "$CODE" 'wal_attribution_retained_materialized_result_is_idle_at_checkpoint' "job runs retained-result idle control"
assert_contains "$CODE" 'tests::wal_attribution_reader_handoff_is_idle_before_materialized_reply' "job selects the exact reader-handoff control"
assert_contains "$CODE" 'reader-handoff attribution command selected zero tests' "job rejects a zero-test reader-handoff invocation"
assert_contains "$CODE" 'slice65_wal reader_handoff_idle_before_reply=passed' "job requires the reader-handoff artifact marker"
assert_absent "$CODE" 'tests::wal_attribution_incident_checkpoint_ladder_retains_typed_erase_observation' "job does not select the perturbing pre-erase raw ladder as primary evidence"
assert_contains "$CODE" 'tests::wal_attribution_actual_checkpoint_observation_retains_real_attempt_facts' "job selects the exact direct-Rust actual-checkpoint control"
assert_contains "$CODE" 'actual-checkpoint observation selected zero tests' "job rejects a zero-test direct-Rust actual-checkpoint invocation"
assert_contains "$CODE" 'slice65_wal actual_checkpoint control=direct_rust phase=before' "job retains direct-Rust pre-attempt facts"
assert_contains "$CODE" 'slice65_wal actual_checkpoint control=direct_rust phase=after' "job retains direct-Rust post-attempt facts"
assert_contains "$CODE" 'slice65_wal actual_checkpoint control=python_serial phase=before' "job retains installed-Python pre-attempt facts"
assert_contains "$CODE" 'slice65_wal actual_checkpoint control=python_serial phase=after' "job retains installed-Python post-attempt facts"
assert_contains "$CODE" 'wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded' "job runs truthful projection-worker record contract"
assert_contains "$CODE" 'tests::wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded' "job selects the truthful full projection-worker lib-test path"
assert_contains "$CODE" 'projection-worker attribution command selected zero tests' "job rejects a zero-test projection-worker invocation"
assert_contains "$CODE" 'projection_worker_original_erase=typed_erasure_incomplete owned_busy_attempts=5' "job requires the original projection-worker typed busy marker"
assert_contains "$CODE" 'projection_worker_completion_ack worker_autocommit=1 collector_roles=idle' "job requires post-finish projection-worker acknowledgement"
assert_contains "$CODE" 'projection_worker_native_state_inventory=state_inventory=complete reason=complete' "job requires complete projection-worker native-state inventory"
assert_contains "$CODE" 'projection_worker_sampler_native_state control=binding_sampler phase=before' "job requires projection-worker sampler before facts"
assert_contains "$CODE" 'projection_worker_sampler_native_state control=binding_sampler phase=after' "job requires projection-worker sampler after facts"
assert_contains "$CODE" 'projection_worker_sampler_terminal outcome=' "job requires projection-worker sampler terminal outcome"
assert_contains "$CODE" 'tests::wal_attribution_post_commit_acknowledges_and_records_raw_checkpoint_diagnostic' "job selects the exact post-commit diagnostic control"
assert_contains "$CODE" 'post-commit diagnostic command selected zero tests' "job rejects a zero-test post-commit diagnostic invocation"
assert_contains "$CODE" 'slice65_wal post_commit_ack direct_inventory=' "job requires direct post-commit transaction facts"
assert_contains "$CODE" 'collector_roles=idle' "job retains collector state separately from direct transaction facts"
assert_contains "$CODE" 'slice65_wal post_commit_raw case=pre_close' "job requires redacted pre-close raw checkpoint samples"
assert_contains "$CODE" 'slice65_wal post_commit_child_raw case=after_close' "job requires the redacted fresh-child probe outcome"
assert_contains "$CODE" 'python_binding_completion_ack reader_autocommit=1 collector=idle' "job requires installed binding direct completion acknowledgement"
assert_absent "$CODE" 'python_binding_completion_ack collector=idle' "job does not accept the collector-only binding completion marker"
assert_contains "$CODE" "\$artifact -SimpleMatch 'slice65_wal python_binding_completion_ack reader_autocommit=1 collector=idle'" "job requires the binding completion acknowledgement in the retained artifact"
assert_absent "$CODE" 'python_binding_snapshot_released' "job does not require the removed binding post-release marker"
assert_contains "$CODE" 'python_binding_direct_inventory=' "job requires direct installed binding inventory"
assert_contains "$CODE" 'python_binding_held_reader_native_state=' "job requires the held reader native-state positive control"
assert_contains "$CODE" 'readers:0(auto=0,txn=read,' "job requires held reader autocommit-off/read-state evidence"
assert_contains "$CODE" 'python_binding_native_state_inventory=state_inventory=complete reason=complete' "job requires post-release complete native-state inventory"
assert_contains "$CODE" 'workers:1(auto=1,txn=none,busy=0,received=1)' "job requires every managed role to be direct-native idle"
assert_contains "$CODE" 'python_binding_native_state control=binding_sampler phase=before' "job requires binding sampler native state before each checkpoint"
assert_contains "$CODE" 'python_binding_native_state control=binding_sampler phase=after' "job requires binding sampler native state after each checkpoint"
assert_contains "$CODE" 'installed binding sampler did not retain complete before/after native-state facts' "job fails closed on incomplete sampler native state"
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
assert_contains "$CODE" 'src/python/pyproject.toml' "job derives the disposable current-wheel version from candidate source"
assert_contains "$CODE" '$currentWheelVersion' "job binds every current-wheel control to the candidate version"
current_wheel_version_controls="$(grep -Fc -- '--wheel-version $currentWheelVersion' <<<"$CODE" || true)"
if [ "$current_wheel_version_controls" -eq 3 ]; then
  pass "job checks all three current-wheel controls against the candidate version"
else
  fail "job has $current_wheel_version_controls current-wheel version controls; expected 3"
fi
released_wheel_version_controls="$(grep -Fc -- '--wheel-version 0.8.22' <<<"$CODE" || true)"
if [ "$released_wheel_version_controls" -eq 1 ]; then
  pass "job keeps the released baseline version separate from the candidate version"
else
  fail "job has $released_wheel_version_controls released-wheel version controls; expected 1"
fi
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
assert_contains "$CODE" 'Select-String -Path $managedReaderLog -SimpleMatch "running 1 test" -Quiet' "job checks managed-reader output for an executed test"
assert_contains "$CODE" '--nocapture' "job retains diagnostic lifecycle markers"
assert_contains "$CODE" 'slice65-windows-wal-attribution.log' "job writes Slice 65 artifact"
assert_contains "$CODE" 'slice65-windows-wal-attribution-${{ github.run_id }}-${{ github.run_attempt }}' "artifact name is unique"
assert_contains "$CODE" 'if: always()' "artifact uploads after a failure"
assert_contains "$CODE" 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a' "artifact uploader is pinned"

for marker in \
  'slice65_wal managed_reader_snapshot_ready' \
  'pause_reader_after_wal_snapshot_for_test' \
  'wal_attribution_checkpoints_for_test' \
  'managed_reader_original_erase=typed_erasure_incomplete owned_busy_attempts=' \
  'managed_reader_completion_ack reader_autocommit=1 collector_roles=idle' \
  'managed_reader_native_state_inventory=' \
  'managed_reader_sampler_native_state' \
  'managed_reader_sampler_terminal outcome=' \
  'active_roles == vec![(WalAttributionRole::ReaderWorker, 0)]' \
  'SELECT COUNT(*) FROM canonical_nodes' \
  'owned_reader_snapshot' \
  'owned_runtime_transaction' \
  'reader_handoff_idle_before_reply=passed' \
  'fn wal_attribution_incident_checkpoint_ladder_retains_typed_erase_observation' \
  '"old_close"' \
  'incident_ladder stage=after_fresh_reads' \
  'incident_ladder runtime_probe_drop_ack' \
  'incident_ladder typed_erase_observation=wal_checkpoint' \
  'incident_ladder erase_observation=' \
  'RuntimeProbeRegistration' \
  'runtime_probe_actual_drop' \
  'retained_materialized_idle=passed' \
  'projection_worker_transaction_ready' \
  'fn wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded' \
  'projection_worker_original_erase=typed_erasure_incomplete owned_busy_attempts=' \
  'projection_worker_completion_ack worker_autocommit=1 collector_roles=idle' \
  'projection_worker_native_state_inventory=' \
  'projection_worker_sampler_native_state' \
  'projection_worker_sampler_terminal outcome=' \
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
assert_contains "$(<"$PY_SOURCE")" \
  '_arm_binding_native_state_observation_for_test' \
  '_drain_binding_native_state_observations_for_test' \
  "installed binding exposes the private binding native-state observer only in its test-hooks build"
assert_contains "$(<"$PY_SOURCE")" \
  '_wal_attribution_binding_native_state_inventory_for_test' \
  'reader_native_state_for_test()' \
  "installed binding exposes complete native-state inventory and held-reader positive evidence"
assert_contains "$(<"$PY_SOURCE")" \
  '_native_raw_wal_checkpoint_for_test' \
  'wrap_pyfunction!(native_raw_wal_checkpoint_for_test, &m)' \
  "installed binding exposes the native child raw-checkpoint hook only in its test-hooks build"
assert_contains "$(<"$PY_SOURCE")" \
  '_arm_actual_checkpoint_observation_for_test' \
  '_drain_actual_checkpoint_observations_for_test' \
  "installed serial exposes actual-checkpoint observation only in its test-hooks build"
assert_contains "$(<"$REPO_ROOT/src/rust/crates/fathomdb-engine/Cargo.toml")" \
  'test-hooks = []' \
  "engine reader rendezvous is a non-default test feature"
assert_contains "$(<"$ENGINE_SOURCE")" \
  '#[cfg(any(debug_assertions, feature = "test-hooks"))]' \
  "managed-reader hook is unavailable from shipped production builds"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'wal_attribution.fire_reader_completion_pause(connection.is_autocommit())' \
  'binding_connection_inventory_for_test' \
  'checkpoint_at_rest_for_test' \
  "engine retains private completion, inventory, and checkpoint sampler seams"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'NativeTransactionState' \
  'native_connection_state_for_test' \
  'binding_native_state_observations' \
  "engine retains private native SQLite transaction and statement-state observation"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'connection.transaction_state(Some("main"))' \
  'connection.is_busy()' \
  'WalNativeStateInventory' \
  "engine uses rusqlite state APIs on owning connections without custom SQLite FFI"
assert_contains "$(<"$ENGINE_SOURCE")" \
  '#[cfg(any(test, feature = "test-hooks"))]' \
  'actual_checkpoint_observations: Mutex<Option<ActualCheckpointObserver>>' \
  "actual checkpoint observation is test/test-hooks-only"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'native_raw_wal_checkpoint_for_test' \
  'open_managed_connection' \
  "engine retains the native fresh-child raw-checkpoint seam behind the audited opener"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'fresh_writer_connection_open=' \
  "source records the live fresh writer connection fact"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'ManagedConnectionCategory' \
  "source classifies every Engine-managed SQLite open"
for marker in \
  'runtime_probe_lifecycle: Mutex<RuntimeProbeLifecycle>' \
  'struct RuntimeProbeLifecycle' \
  'struct RuntimeProbeRegistration' \
  'struct RuntimeProbeConnection' \
  'fn register_runtime_probe' \
  'impl RuntimeProbeRegistration' \
  'impl Drop for RuntimeProbeRegistration' \
  'impl RuntimeProbeConnection'; do
  assert_cfg_test_near_marker \
    "$ENGINE_SOURCE" \
    "$marker" \
    "runtime-probe ladder bookkeeping is cfg(test)-only: $marker"
done
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
assert_absent "$(<"$PY_CONTROL")" \
  'import sqlite3' \
  "installed binding diagnostic never imports Python stdlib sqlite3"
assert_absent "$(<"$PY_CONTROL")" \
  'sqlite3.connect' \
  "installed binding diagnostic never opens a stdlib raw checkpoint connection"
serial_incident_body="$(python_function_body "$PY_CONTROL" "run_serial_incident")"
assert_absent "$serial_incident_body" \
  'fresh.transition' \
  "installed serial has no deletion transition after its one original erase"
assert_absent "$serial_incident_body" \
  'fresh.purge' \
  "installed serial has no purge after its one original erase"
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
  'completion_pause.reader_connection_autocommit_for_test()' \
  'python_binding_completion_ack reader_autocommit=1 collector=idle' \
  "binding diagnostic verifies actual reader autocommit before completion acknowledgement"
assert_before_in_python_function \
  "$PY_CONTROL" \
  "run_binding_reader_erase" \
  'python_binding_completion_ack reader_autocommit=1 collector=idle' \
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
  'samples = test_hooks._checkpoint_at_rest_for_test()' \
  "binding diagnostic places the Engine sampler after raw sample one"
assert_before_in_python_function \
  "$PY_CONTROL" \
  "run_binding_reader_erase" \
  'samples = test_hooks._checkpoint_at_rest_for_test()' \
  '_raw_binding_checkpoint(path, "after_engine_sampler")' \
  "binding diagnostic places raw sample two after the Engine sampler"
binding_erase_calls="$(python_function_body "$PY_CONTROL" "run_binding_reader_erase" | grep -Fc 'engine.erase_source(' || true)"
if [ "$binding_erase_calls" -eq 1 ]; then
  pass "binding diagnostic retains exactly one original erasure"
else
  fail "binding diagnostic must retain exactly one original erasure; found $binding_erase_calls"
fi
binding_body="$(python_function_body "$PY_CONTROL" "run_binding_reader_erase")"
assert_contains "$binding_body" \
  'all(role_fact in before for role_fact in NATIVE_IDLE_ROLE_FACTS)' \
  "every sampler before record carries all twelve native role facts"
assert_contains "$binding_body" \
  'all(role_fact in after for role_fact in NATIVE_IDLE_ROLE_FACTS)' \
  "every sampler after record carries all twelve native role facts"
assert_contains "$binding_body" \
  'if first_raw[0] != 0 or second_raw[0] != 0:' \
  "binding diagnostic runs the child only after a busy raw sample"
assert_before_in_text \
  "$binding_body" \
  'engine.close()' \
  'subprocess.run(' \
  "binding diagnostic closes and joins its Engine before the conditional child"
binding_child_calls="$(grep -Fc 'subprocess.run(' <<<"$binding_body" || true)"
if [ "$binding_child_calls" -eq 1 ]; then
  pass "binding diagnostic retains exactly one conditional fresh child"
else
  fail "binding diagnostic must retain exactly one conditional fresh child; found $binding_child_calls"
fi
binding_child_body="$(python_function_body "$PY_CONTROL" "run_binding_child")"
assert_contains "$binding_child_body" \
  '_native_raw_checkpoint_test_hook()._native_raw_wal_checkpoint_for_test(path)' \
  "fresh child calls the native FathomDB/Rusqlite checkpoint hook"
actual_checkpoint_body="$(function_body "$ENGINE_SOURCE" "wal_attribution_actual_checkpoint_observation_retains_real_attempt_facts")"
assert_contains "$actual_checkpoint_body" \
  'arm_actual_checkpoint_observation_for_test' \
  "direct-Rust actual-checkpoint control arms the private observer"
assert_contains "$actual_checkpoint_body" \
  'fresh.engine.erase_source(' \
  "direct-Rust actual-checkpoint control retains one original erase"
assert_before_in_text \
  "$actual_checkpoint_body" \
  'arm_actual_checkpoint_observation_for_test' \
  'fresh.engine.erase_source(' \
  "direct-Rust observer arms immediately before the original erase"
assert_absent "$actual_checkpoint_body" \
  'incident_ladder_raw_checkpoint' \
  "direct-Rust actual-checkpoint control has no raw TRUNCATE probe"
assert_absent "$actual_checkpoint_body" \
  'RuntimeProbe' \
  "direct-Rust actual-checkpoint control has no RuntimeProbe"
assert_absent "$actual_checkpoint_body" \
  'wal_checkpoint_truncate_once' \
  "direct-Rust actual-checkpoint control has no extra checkpoint call"
managed_reader_body="$(function_body "$ENGINE_SOURCE" "wal_attribution_owned_reader_typed_refusal_then_post_release_sampler_is_recorded")"
managed_reader_erase_calls="$(grep -Fc 'opened.engine.erase_source(' <<<"$managed_reader_body" || true)"
if [ "$managed_reader_erase_calls" -eq 1 ]; then
  pass "managed-reader diagnostic retains exactly one original erase"
else
  fail "managed-reader diagnostic must retain exactly one original erase; found $managed_reader_erase_calls"
fi
assert_contains "$managed_reader_body" \
  'arm_next_reader_completion_pause_for_test' \
  "managed-reader diagnostic waits for post-finish owner-thread acknowledgement"
assert_contains "$managed_reader_body" \
  'native_state_inventory_for_test' \
  "managed-reader diagnostic requires complete native-state inventory"
assert_contains "$managed_reader_body" \
  'state_inventory=incomplete' \
  "managed-reader diagnostic emits incomplete inventory before failure"
assert_contains "$managed_reader_body" \
  'arm_binding_native_state_observation_for_test' \
  "managed-reader diagnostic arms the bounded sampler observer"
assert_contains "$managed_reader_body" \
  'checkpoint_at_rest_for_test' \
  "managed-reader diagnostic runs exactly one bounded sampler"
assert_absent "$managed_reader_body" \
  'retry after release' \
  "managed-reader diagnostic does not retry the original erase"
projection_worker_body="$(function_body "$ENGINE_SOURCE" "wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded")"
projection_worker_erase_calls="$(grep -Fc 'opened.engine.complete_erasure_at_rest(' <<<"$projection_worker_body" || true)"
if [ "$projection_worker_erase_calls" -eq 1 ]; then
  pass "projection-worker diagnostic retains exactly one original erase"
else
  fail "projection-worker diagnostic must retain exactly one original erase; found $projection_worker_erase_calls"
fi
assert_contains "$projection_worker_body" \
  'report_runtime_connection_inventory_for_test' \
  "projection-worker diagnostic waits for post-finish owner-thread acknowledgement"
assert_contains "$projection_worker_body" \
  'native_state_inventory_for_test' \
  "projection-worker diagnostic requires complete native-state inventory"
assert_contains "$projection_worker_body" \
  'state_inventory=incomplete' \
  "projection-worker diagnostic emits incomplete inventory before failure"
assert_contains "$projection_worker_body" \
  'arm_binding_native_state_observation_for_test' \
  "projection-worker diagnostic arms the bounded sampler observer"
assert_contains "$projection_worker_body" \
  'checkpoint_at_rest_for_test' \
  "projection-worker diagnostic runs exactly one bounded sampler"
assert_absent "$projection_worker_body" \
  'retry after release' \
  "projection-worker diagnostic does not retry the original erase"
actual_checkpoint_engine_body="$(function_body "$ENGINE_SOURCE" "complete_erasure_at_rest")"
assert_before_in_text \
  "$actual_checkpoint_engine_body" \
  'self.actual_checkpoint_observation_for_test(' \
  'let checkpoint_result = self.wal_checkpoint_truncate_once(false);' \
  "actual observer records immediately before the existing checkpoint call"
assert_before_in_text \
  "$actual_checkpoint_engine_body" \
  'let checkpoint_result = self.wal_checkpoint_truncate_once(false);' \
  'checkpoint_result.as_ref().ok().cloned()' \
  "actual observer records immediately after the existing checkpoint call"
normal_runtime_inventory_body="$(function_body "$ENGINE_SOURCE" "report_runtime_connection_inventory_for_test")"
assert_contains "$(<"$ENGINE_SOURCE")" \
  'respond: SyncSender<(WalAttributionRole, usize, bool)>' \
  "normal actual/post-commit runtime inventory retains boolean replies"
assert_contains "$normal_runtime_inventory_body" \
  'Duration::from_secs(2)' \
  "normal actual/post-commit runtime inventory retains its two-second timeout"
assert_absent "$normal_runtime_inventory_body" \
  'native_connection_state_for_test' \
  "normal actual/post-commit runtime inventory cannot sample native state"
assert_absent "$normal_runtime_inventory_body" \
  'NativeConnectionStateFact' \
  "normal actual/post-commit runtime inventory cannot return native state facts"
native_runtime_inventory_body="$(function_body "$ENGINE_SOURCE" "report_runtime_native_state_inventory_for_test")"
assert_contains "$native_runtime_inventory_body" \
  'Duration::from_millis(250)' \
  "binding native-state runtime inventory has its own finite timeout"
assert_contains "$native_runtime_inventory_body" \
  'NativeConnectionStateFact' \
  "binding native-state runtime inventory has a separate fact reply"
native_state_inventory_body="$(function_body "$ENGINE_SOURCE" "native_state_inventory_for_test")"
assert_contains "$native_state_inventory_body" \
  'report_runtime_native_state_inventory_for_test' \
  "binding native-state inventory alone requests runtime native facts"
actual_direct_inventory_body="$(function_body "$ENGINE_SOURCE" "actual_checkpoint_direct_inventory_for_test")"
assert_contains "$actual_direct_inventory_body" \
  'report_runtime_connection_inventory_for_test' \
  "normal actual observer retains its boolean runtime inventory request"
assert_absent "$actual_direct_inventory_body" \
  'report_runtime_native_state_inventory_for_test' \
  "normal actual observer cannot request runtime native facts"
serial_incident_body="$(python_function_body "$PY_CONTROL" "run_serial_incident")"
assert_contains "$serial_incident_body" \
  'test_hooks._arm_actual_checkpoint_observation_for_test()' \
  "installed Python serial arms the private observer immediately before erase"
assert_contains "$serial_incident_body" \
  'test_hooks._drain_actual_checkpoint_observations_for_test()' \
  "installed Python serial drains real-attempt records after erase"
assert_before_in_text \
  "$serial_incident_body" \
  'test_hooks._arm_actual_checkpoint_observation_for_test()' \
  'nested = fresh.erase_source(' \
  "installed Python observer arms immediately before its original erase"
assert_absent "$serial_incident_body" \
  '_native_raw_wal_checkpoint_for_test' \
  "installed Python serial has no raw TRUNCATE probe"
assert_absent "$serial_incident_body" \
  '_checkpoint_at_rest_for_test' \
  "installed Python serial has no extra Engine checkpoint sampler"
assert_absent "$serial_incident_body" \
  '_arm_binding_native_state_observation_for_test' \
  "installed Python serial never arms the binding-only native-state observer"
assert_absent "$actual_checkpoint_engine_body" \
  'binding_native_state_observation_for_test' \
  "real erasure checkpoint path is untouched by the binding sampler observer"
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

  for marker in \
    'managed_reader_completion_ack reader_autocommit=1 collector_roles=idle' \
    'managed_reader_native_state_inventory=state_inventory=complete reason=complete' \
    'managed_reader_sampler_native_state control=binding_sampler phase=before' \
    'managed_reader_sampler_native_state control=binding_sampler phase=after' \
    'managed_reader_sampler_terminal outcome='; do
    MANAGED_MARKER_MUTATED="$TMPROOT/ci-without-managed-marker.yml"
    sed "s|$marker|managed-marker-removed|" "$CI" >"$MANAGED_MARKER_MUTATED"
    set +e
    managed_marker_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$MANAGED_MARKER_MUTATED" bash "$0" 2>&1)"
    managed_marker_rc=$?
    set -e
    if [ "$managed_marker_rc" -ne 0 ] && grep -Fq "missing: $marker" <<<"$managed_marker_out"; then
      pass "mutation proves managed-reader artifact marker is load-bearing: $marker"
    else
      fail "mutation did not fail managed-reader artifact marker: $marker: $managed_marker_out"
    fi
  done

  MANAGED_READER_REERASE_MUTATED="$TMPROOT/lib-with-managed-reader-second-erase.rs"
  sed 's/let samples = opened/let _second = opened.engine.erase_source("slice65-owned-reader");\n        let samples = opened/' "$ENGINE_SOURCE" \
    >"$MANAGED_READER_REERASE_MUTATED"
  set +e
  managed_reader_reerase_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$MANAGED_READER_REERASE_MUTATED" bash "$0" 2>&1)"
  managed_reader_reerase_rc=$?
  set -e
  if [ "$managed_reader_reerase_rc" -ne 0 ] \
    && grep -Fq 'managed-reader diagnostic must retain exactly one original erase; found 2' <<<"$managed_reader_reerase_out"; then
    pass "mutation proves managed-reader single-erase guard is load-bearing"
  else
    fail "mutation did not fail managed-reader single-erase guard: $managed_reader_reerase_out"
  fi

  MANAGED_READER_RETRY_MUTATED="$TMPROOT/lib-with-managed-reader-retry-marker.rs"
  sed 's/let samples = opened/\/\/ retry after release\n        let samples = opened/' "$ENGINE_SOURCE" \
    >"$MANAGED_READER_RETRY_MUTATED"
  set +e
  managed_reader_retry_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$MANAGED_READER_RETRY_MUTATED" bash "$0" 2>&1)"
  managed_reader_retry_rc=$?
  set -e
  if [ "$managed_reader_retry_rc" -ne 0 ] \
    && grep -Fq 'managed-reader diagnostic does not retry the original erase (unexpected: retry after release)' <<<"$managed_reader_retry_out"; then
    pass "mutation proves managed-reader no-retry guard is load-bearing"
  else
    fail "mutation did not fail managed-reader no-retry guard: $managed_reader_retry_out"
  fi

  for marker in \
    'projection_worker_original_erase=typed_erasure_incomplete owned_busy_attempts=5' \
    'projection_worker_completion_ack worker_autocommit=1 collector_roles=idle' \
    'projection_worker_native_state_inventory=state_inventory=complete reason=complete' \
    'projection_worker_sampler_native_state control=binding_sampler phase=before' \
    'projection_worker_sampler_native_state control=binding_sampler phase=after' \
    'projection_worker_sampler_terminal outcome='; do
    PROJECTION_MARKER_MUTATED="$TMPROOT/ci-without-projection-marker.yml"
    sed "s|$marker|projection-marker-removed|" "$CI" >"$PROJECTION_MARKER_MUTATED"
    set +e
    projection_marker_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$PROJECTION_MARKER_MUTATED" bash "$0" 2>&1)"
    projection_marker_rc=$?
    set -e
    if [ "$projection_marker_rc" -ne 0 ] && grep -Fq "missing: $marker" <<<"$projection_marker_out"; then
      pass "mutation proves projection-worker artifact marker is load-bearing: $marker"
    else
      fail "mutation did not fail projection-worker artifact marker: $marker: $projection_marker_out"
    fi
  done

  PROJECTION_SELECTOR_MUTATED="$TMPROOT/ci-without-projection-selector.yml"
  sed 's/tests::wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded/tests::projection-worker-selector-removed/' "$CI" >"$PROJECTION_SELECTOR_MUTATED"
  set +e
  projection_selector_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$PROJECTION_SELECTOR_MUTATED" bash "$0" 2>&1)"
  projection_selector_rc=$?
  set -e
  if [ "$projection_selector_rc" -ne 0 ] \
    && grep -Fq 'job selects the truthful full projection-worker lib-test path (missing: tests::wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded)' <<<"$projection_selector_out"; then
    pass "mutation proves projection-worker exact selector is load-bearing"
  else
    fail "mutation did not fail projection-worker selector assertion: $projection_selector_out"
  fi

  PROJECTION_ZERO_TEST_MUTATED="$TMPROOT/ci-without-projection-zero-test-guard.yml"
  sed 's/projection-worker attribution command selected zero tests/projection-worker-zero-test-guard-removed/' "$CI" >"$PROJECTION_ZERO_TEST_MUTATED"
  set +e
  projection_zero_test_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$PROJECTION_ZERO_TEST_MUTATED" bash "$0" 2>&1)"
  projection_zero_test_rc=$?
  set -e
  if [ "$projection_zero_test_rc" -ne 0 ] \
    && grep -Fq 'job rejects a zero-test projection-worker invocation (missing: projection-worker attribution command selected zero tests)' <<<"$projection_zero_test_out"; then
    pass "mutation proves projection-worker zero-test guard is load-bearing"
  else
    fail "mutation did not fail projection-worker zero-test guard: $projection_zero_test_out"
  fi

  PROJECTION_REERASE_MUTATED="$TMPROOT/lib-with-projection-second-erase.rs"
  sed 's/let samples = opened/let _second = opened.engine.complete_erasure_at_rest("slice65-projection-checkpoint");\n        let samples = opened/' "$ENGINE_SOURCE" \
    >"$PROJECTION_REERASE_MUTATED"
  set +e
  projection_reerase_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$PROJECTION_REERASE_MUTATED" bash "$0" 2>&1)"
  projection_reerase_rc=$?
  set -e
  if [ "$projection_reerase_rc" -ne 0 ] \
    && grep -Fq 'projection-worker diagnostic must retain exactly one original erase; found 2' <<<"$projection_reerase_out"; then
    pass "mutation proves projection-worker single-erase guard is load-bearing"
  else
    fail "mutation did not fail projection-worker single-erase guard: $projection_reerase_out"
  fi

  PROJECTION_RETRY_MUTATED="$TMPROOT/lib-with-projection-retry-marker.rs"
  sed 's/let samples = opened/\/\/ retry after release\n        let samples = opened/' "$ENGINE_SOURCE" \
    >"$PROJECTION_RETRY_MUTATED"
  set +e
  projection_retry_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$PROJECTION_RETRY_MUTATED" bash "$0" 2>&1)"
  projection_retry_rc=$?
  set -e
  if [ "$projection_retry_rc" -ne 0 ] \
    && grep -Fq 'projection-worker diagnostic does not retry the original erase (unexpected: retry after release)' <<<"$projection_retry_out"; then
    pass "mutation proves projection-worker no-retry guard is load-bearing"
  else
    fail "mutation did not fail projection-worker no-retry guard: $projection_retry_out"
  fi

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
  sed '/Select-String -Path \$managedReaderLog -SimpleMatch "running 1 test" -Quiet/d;/managed-reader attribution command selected zero tests/d' "$CI" \
    >"$EXECUTION_GUARD_MUTATED"
  set +e
  execution_guard_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$EXECUTION_GUARD_MUTATED" bash "$0" 2>&1)"
  execution_guard_rc=$?
  set -e
  if [ "$execution_guard_rc" -ne 0 ] \
    && grep -Fq 'job checks managed-reader output for an executed test (missing: Select-String -Path $managedReaderLog -SimpleMatch "running 1 test" -Quiet)' <<<"$execution_guard_out"; then
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

  ACTUAL_CHECKPOINT_SELECTOR_MUTATED="$TMPROOT/ci-without-actual-checkpoint-selector.yml"
  sed '/tests::wal_attribution_actual_checkpoint_observation_retains_real_attempt_facts/d' "$CI" \
    >"$ACTUAL_CHECKPOINT_SELECTOR_MUTATED"
  set +e
  actual_checkpoint_selector_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$ACTUAL_CHECKPOINT_SELECTOR_MUTATED" bash "$0" 2>&1)"
  actual_checkpoint_selector_rc=$?
  set -e
  if [ "$actual_checkpoint_selector_rc" -ne 0 ] \
    && grep -Fq 'job selects the exact direct-Rust actual-checkpoint control (missing: tests::wal_attribution_actual_checkpoint_observation_retains_real_attempt_facts)' <<<"$actual_checkpoint_selector_out"; then
    pass "mutation proves actual-checkpoint selector assertion is load-bearing"
  else
    fail "mutation did not fail actual-checkpoint selector assertion: $actual_checkpoint_selector_out"
  fi

  ACTUAL_CHECKPOINT_GUARD_MUTATED="$TMPROOT/ci-without-actual-checkpoint-guard.yml"
  sed '/actual-checkpoint observation selected zero tests/d' "$CI" >"$ACTUAL_CHECKPOINT_GUARD_MUTATED"
  set +e
  actual_checkpoint_guard_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$ACTUAL_CHECKPOINT_GUARD_MUTATED" bash "$0" 2>&1)"
  actual_checkpoint_guard_rc=$?
  set -e
  if [ "$actual_checkpoint_guard_rc" -ne 0 ] \
    && grep -Fq 'job rejects a zero-test direct-Rust actual-checkpoint invocation (missing: actual-checkpoint observation selected zero tests)' <<<"$actual_checkpoint_guard_out"; then
    pass "mutation proves actual-checkpoint zero-test guard is load-bearing"
  else
    fail "mutation did not fail actual-checkpoint zero-test guard: $actual_checkpoint_guard_out"
  fi

  ACTUAL_CHECKPOINT_ARTIFACT_MUTATED="$TMPROOT/ci-without-actual-checkpoint-artifact.yml"
  sed '/slice65_wal actual_checkpoint control=direct_rust phase=before/d' "$CI" >"$ACTUAL_CHECKPOINT_ARTIFACT_MUTATED"
  set +e
  actual_checkpoint_artifact_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$ACTUAL_CHECKPOINT_ARTIFACT_MUTATED" bash "$0" 2>&1)"
  actual_checkpoint_artifact_rc=$?
  set -e
  if [ "$actual_checkpoint_artifact_rc" -ne 0 ] \
    && grep -Fq 'job retains direct-Rust pre-attempt facts (missing: slice65_wal actual_checkpoint control=direct_rust phase=before)' <<<"$actual_checkpoint_artifact_out"; then
    pass "mutation proves actual-checkpoint artifact assertion is load-bearing"
  else
    fail "mutation did not fail actual-checkpoint artifact assertion: $actual_checkpoint_artifact_out"
  fi

  ACTUAL_CHECKPOINT_RAW_MUTATED="$TMPROOT/lib-with-raw-primary-probe.rs"
  sed 's/let observed = fresh.engine.erase_source(/let _raw_probe = incident_ladder_raw_checkpoint;\n        let observed = fresh.engine.erase_source(/' "$ENGINE_SOURCE" >"$ACTUAL_CHECKPOINT_RAW_MUTATED"
  set +e
  actual_checkpoint_raw_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$ACTUAL_CHECKPOINT_RAW_MUTATED" bash "$0" 2>&1)"
  actual_checkpoint_raw_rc=$?
  set -e
  if [ "$actual_checkpoint_raw_rc" -ne 0 ] \
    && grep -Fq 'direct-Rust actual-checkpoint control has no raw TRUNCATE probe (unexpected: incident_ladder_raw_checkpoint)' <<<"$actual_checkpoint_raw_out"; then
    pass "mutation proves raw-probe exclusion is load-bearing"
  else
    fail "mutation did not fail raw-probe exclusion: $actual_checkpoint_raw_out"
  fi

  ACTUAL_CHECKPOINT_RUNTIME_MUTATED="$TMPROOT/lib-with-runtime-primary-probe.rs"
  sed 's/let observed = fresh.engine.erase_source(/let _runtime_probe = RuntimeProbeConnection;\n        let observed = fresh.engine.erase_source(/' "$ENGINE_SOURCE" >"$ACTUAL_CHECKPOINT_RUNTIME_MUTATED"
  set +e
  actual_checkpoint_runtime_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$ACTUAL_CHECKPOINT_RUNTIME_MUTATED" bash "$0" 2>&1)"
  actual_checkpoint_runtime_rc=$?
  set -e
  if [ "$actual_checkpoint_runtime_rc" -ne 0 ] \
    && grep -Fq 'direct-Rust actual-checkpoint control has no RuntimeProbe (unexpected: RuntimeProbe)' <<<"$actual_checkpoint_runtime_out"; then
    pass "mutation proves RuntimeProbe exclusion is load-bearing"
  else
    fail "mutation did not fail RuntimeProbe exclusion: $actual_checkpoint_runtime_out"
  fi

  RUNTIME_PROBE_LIFECYCLE_MUTATED="$TMPROOT/lib-without-runtime-probe-registration.rs"
  sed 's/RuntimeProbeRegistration/ProbeLifecycleRegistrationRemoved/g' "$ENGINE_SOURCE" >"$RUNTIME_PROBE_LIFECYCLE_MUTATED"
  set +e
  runtime_probe_lifecycle_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$RUNTIME_PROBE_LIFECYCLE_MUTATED" bash "$0" 2>&1)"
  runtime_probe_lifecycle_rc=$?
  set -e
  if [ "$runtime_probe_lifecycle_rc" -ne 0 ] \
    && grep -Fq 'source retains RuntimeProbeRegistration (missing: RuntimeProbeRegistration)' <<<"$runtime_probe_lifecycle_out"; then
    pass "mutation proves runtime-probe lifecycle assertion is load-bearing"
  else
    fail "mutation did not fail runtime-probe lifecycle assertion: $runtime_probe_lifecycle_out"
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

  ARTIFACT_BINDING_COMPLETION_MUTATED="$TMPROOT/ci-without-artifact-binding-completion.yml"
  sed '/\$artifact -SimpleMatch .*python_binding_completion_ack reader_autocommit=1 collector=idle/d' "$CI" >"$ARTIFACT_BINDING_COMPLETION_MUTATED"
  set +e
  artifact_binding_completion_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$ARTIFACT_BINDING_COMPLETION_MUTATED" bash "$0" 2>&1)"
  artifact_binding_completion_rc=$?
  set -e
  if [ "$artifact_binding_completion_rc" -ne 0 ] \
    && grep -Fq "job requires the binding completion acknowledgement in the retained artifact (missing: \$artifact -SimpleMatch 'slice65_wal python_binding_completion_ack reader_autocommit=1 collector=idle')" <<<"$artifact_binding_completion_out"; then
    pass "mutation proves retained artifact binding-completion assertion is load-bearing"
  else
    fail "mutation did not fail retained artifact binding-completion assertion: $artifact_binding_completion_out"
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

  RETAINED_ORDER_MUTATED="$TMPROOT/ci-without-retained-command.yml"
  sed '/--control retained --wheel-version/d' "$CI" >"$RETAINED_ORDER_MUTATED"
  set +e
  retained_order_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$RETAINED_ORDER_MUTATED" bash "$0" 2>&1)"
  retained_order_rc=$?
  set -e
  if [ "$retained_order_rc" -ne 0 ] \
    && grep -Fq 'installed binding diagnostic runs before retained-result control (expected --control binding before --control retained)' <<<"$retained_order_out"; then
    pass "mutation proves installed binding-to-retained ordering is load-bearing"
  else
    fail "mutation did not fail installed binding-to-retained ordering assertion: $retained_order_out"
  fi

  NATIVE_STATE_ROLE_MUTATED="$TMPROOT/ci-without-native-state-worker-role.yml"
  sed 's/workers:1(auto=1,txn=none,busy=0,received=1)/workers:1(native-state-role-removed)/' "$CI" \
    >"$NATIVE_STATE_ROLE_MUTATED"
  set +e
  native_state_role_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$NATIVE_STATE_ROLE_MUTATED" bash "$0" 2>&1)"
  native_state_role_rc=$?
  set -e
  if [ "$native_state_role_rc" -ne 0 ] \
    && grep -Fq 'job requires every managed role to be direct-native idle (missing: workers:1(auto=1,txn=none,busy=0,received=1))' <<<"$native_state_role_out"; then
    pass "mutation proves native-state role fact is load-bearing"
  else
    fail "mutation did not fail native-state role fact assertion: $native_state_role_out"
  fi

  NATIVE_STATE_MARKER_MUTATED="$TMPROOT/ci-without-native-state-before-marker.yml"
  sed '/python_binding_native_state control=binding_sampler phase=before/d' "$CI" >"$NATIVE_STATE_MARKER_MUTATED"
  set +e
  native_state_marker_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 CI_YML="$NATIVE_STATE_MARKER_MUTATED" bash "$0" 2>&1)"
  native_state_marker_rc=$?
  set -e
  if [ "$native_state_marker_rc" -ne 0 ] \
    && grep -Fq 'job requires binding sampler native state before each checkpoint (missing: python_binding_native_state control=binding_sampler phase=before)' <<<"$native_state_marker_out"; then
    pass "mutation proves native-state before marker is load-bearing"
  else
    fail "mutation did not fail native-state before marker assertion: $native_state_marker_out"
  fi

  SERIAL_NATIVE_STATE_MUTATED="$TMPROOT/installed-control-with-binding-observer-in-serial.py"
  sed 's/test_hooks\._arm_actual_checkpoint_observation_for_test()/test_hooks._arm_binding_native_state_observation_for_test()\n                test_hooks._arm_actual_checkpoint_observation_for_test()/' "$PY_CONTROL" \
    >"$SERIAL_NATIVE_STATE_MUTATED"
  set +e
  serial_native_state_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 PY_CONTROL="$SERIAL_NATIVE_STATE_MUTATED" bash "$0" 2>&1)"
  serial_native_state_rc=$?
  set -e
  if [ "$serial_native_state_rc" -ne 0 ] \
    && grep -Fq 'installed Python serial never arms the binding-only native-state observer (unexpected: _arm_binding_native_state_observation_for_test)' <<<"$serial_native_state_out"; then
    pass "mutation proves normal-serial exclusion is load-bearing"
  else
    fail "mutation did not fail normal-serial exclusion assertion: $serial_native_state_out"
  fi

  ACTUAL_NATIVE_STATE_MUTATED="$TMPROOT/lib-with-native-runtime-request-in-actual-observer.rs"
  sed 's/let runtime = self\.projection_runtime\.report_runtime_connection_inventory_for_test();/let runtime = self.projection_runtime.report_runtime_native_state_inventory_for_test();/' "$ENGINE_SOURCE" \
    >"$ACTUAL_NATIVE_STATE_MUTATED"
  set +e
  actual_native_state_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$ACTUAL_NATIVE_STATE_MUTATED" bash "$0" 2>&1)"
  actual_native_state_rc=$?
  set -e
  if [ "$actual_native_state_rc" -ne 0 ] \
    && grep -Fq 'normal actual observer cannot request runtime native facts (unexpected: report_runtime_native_state_inventory_for_test)' <<<"$actual_native_state_out"; then
    pass "mutation proves normal actual observer cannot reach native state"
  else
    fail "mutation did not fail normal actual/native-state isolation: $actual_native_state_out"
  fi

  RUNTIME_TIMEOUT_MUTATED="$TMPROOT/lib-with-short-normal-runtime-timeout.rs"
  sed 's/recv_timeout(Duration::from_secs(2))/recv_timeout(Duration::from_millis(250))/' "$ENGINE_SOURCE" \
    >"$RUNTIME_TIMEOUT_MUTATED"
  set +e
  runtime_timeout_out="$(WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 ENGINE_SOURCE="$RUNTIME_TIMEOUT_MUTATED" bash "$0" 2>&1)"
  runtime_timeout_rc=$?
  set -e
  if [ "$runtime_timeout_rc" -ne 0 ] \
    && grep -Fq 'normal actual/post-commit runtime inventory retains its two-second timeout (missing: Duration::from_secs(2))' <<<"$runtime_timeout_out"; then
    pass "mutation proves normal runtime timeout is load-bearing"
  else
    fail "mutation did not fail normal runtime timeout assertion: $runtime_timeout_out"
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
