#!/usr/bin/env bash
# 0.8.28 Slice 31 (T-CUDA-COMPAT C1, AC31-15): every PTX module a FathomDB CUDA
# build carries must be loadable by the oldest driver the 12.6 artifact
# supports (PTX ISA <= 8.5, CUDA 12.6) and target exactly the contract's
# compute capability (sm_75 on x86_64, sm_87 on the Jetson Orin).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-ptx-driver-floor.py"
FIXTURES="$REPO_ROOT/scripts/tests/fixtures/ptx-floor"

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

[ -f "$CHECKER" ] || fail "checker scripts/check-ptx-driver-floor.py is absent"

run_checker() {
  set +e
  OUT="$(python3 "$CHECKER" "$@" 2>&1)"
  RC=$?
  set -e
}

expect_pass() {
  local description="$1"
  shift
  run_checker "$@"
  [ "$RC" -eq 0 ] || fail "$description — expected exit 0, got rc=$RC: $OUT"
  pass "$description"
}

expect_fail_with() {
  local expected="$1" description="$2"
  shift 2
  run_checker "$@"
  [ "$RC" -eq 1 ] || fail "$description — expected exit 1, got rc=$RC: $OUT"
  grep -Fq -- "$expected" <<<"$OUT" || fail "$description — output did not name '$expected': $OUT"
  pass "$description"
}

expect_pass 'ptx_8_5_sm_75_passes' --ptx-dir "$FIXTURES/ok-sm75" --target sm_75
grep -Fq '2 PTX module(s)' <<<"$OUT" || fail "ptx_8_5_sm_75_passes must report the module count: $OUT"
expect_fail_with 'fill.ptx: .version 8.6 is above the driver floor 8.5' 'ptx_8_6_fails' \
  --ptx-dir "$FIXTURES/version-8-6" --target sm_75
expect_fail_with 'affine.ptx: .target sm_80 is not sm_75' 'ptx_target_sm_80_fails_for_x86' \
  --ptx-dir "$FIXTURES/target-sm80" --target sm_75
expect_pass 'ptx_target_sm_87_passes_for_tegra' --ptx-dir "$FIXTURES/tegra-sm87" --target sm_87
expect_fail_with 'no PTX module found' 'no_ptx_found_fails' --ptx-dir "$FIXTURES/empty" --target sm_75
