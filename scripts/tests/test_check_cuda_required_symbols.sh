#!/usr/bin/env bash
# 0.8.28 Slice 31 (owner ruling Q26, design § 9.1 "Drift guard"): the cuBLAS
# and cuRAND symbols D-18 requires before building a device must be exactly
# the sys functions cudarc's result layers call, and neither Candle nor
# FathomDB may call a sys function (or a safe-layer method outside the result
# layer) directly. A pin move that changes the set fails here until
# cuda_libraries.rs is updated.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-cuda-required-symbols.py"
FIXTURES="$REPO_ROOT/scripts/tests/fixtures/cuda-required-symbols"

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

[ -f "$CHECKER" ] || fail "checker scripts/check-cuda-required-symbols.py is absent"

run_fixture() {
  local dir="$FIXTURES/$1"
  set +e
  OUT="$(python3 "$CHECKER" --cudarc-src "$dir/cudarc" --candle-src "$dir/candle" \
    --fathomdb-src "$dir/fathomdb" --constants "$dir/cuda_libraries.rs" 2>&1)"
  RC=$?
  set -e
}

run_fixture matching
[ "$RC" -eq 0 ] || fail "matching_sets_pass — expected exit 0, got rc=$RC: $OUT"
pass 'matching_sets_pass'

run_fixture result-symbol-missing
if [ "$RC" -ne 1 ] || ! grep -Fq 'cuBLAS: called by the result layer but not in REQUIRED_CUBLAS_SYMBOLS: cublasSnrm2_v2' <<<"$OUT"; then
  fail "a_result_layer_symbol_missing_from_the_constant_fails — rc=$RC: $OUT"
fi
pass 'a_result_layer_symbol_missing_from_the_constant_fails'

run_fixture candle-sys-call
if [ "$RC" -ne 1 ] || ! grep -Fq 'Candle calls cublasSgemm_v2 directly' <<<"$OUT"; then
  fail "a_direct_candle_sys_call_fails — rc=$RC: $OUT"
fi
pass 'a_direct_candle_sys_call_fails'

run_fixture fathomdb-sys-call
if [ "$RC" -ne 1 ] || ! grep -Fq 'FathomDB calls curandCreateGenerator directly' <<<"$OUT"; then
  fail "a_fathomdb_sys_call_fails — rc=$RC: $OUT"
fi
pass 'a_fathomdb_sys_call_fails'

# The real tree: the resolved cudarc-fathomdb and Candle fork sources from
# `cargo metadata --offline --locked`, and the embedder's constants.
(cd "$REPO_ROOT" && cargo fetch --locked >/dev/null 2>&1) || fail "cargo fetch --locked failed on the real tree"
set +e
OUT="$(python3 "$CHECKER" --root "$REPO_ROOT" 2>&1)"
RC=$?
set -e
[ "$RC" -eq 0 ] || fail "the real tree's required symbol sets must match cudarc's result layers: $OUT"
pass "the real tree: $OUT"
