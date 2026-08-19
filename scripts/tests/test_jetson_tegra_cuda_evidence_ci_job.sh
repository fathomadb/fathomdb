#!/usr/bin/env bash
# Slice 80: retain the manual-only Jetson CUDA evidence route.  The actual
# proof must run on the self-hosted Tegra runner; this fixture makes a Linux
# edit unable to silently broaden, de-harden, or remove that route.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKFLOW="${JETSON_TEGRA_CI_YML:-$REPO_ROOT/.github/workflows/jetson-tegra-cuda-evidence.yml}"
ACTIONLINT_CONFIG="${ACTIONLINT_CONFIG:-$REPO_ROOT/.github/actionlint.yaml}"
PASSED=0
FAILED=0

pass() { printf 'PASS  %s\n' "$1"; PASSED=$((PASSED + 1)); }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

assert_contains() {
  local needle="$1" description="$2"
  if grep -Fq -- "$needle" "$WORKFLOW"; then
    pass "$description"
  else
    fail "$description (missing: $needle)"
  fi
}

assert_absent() {
  local needle="$1" description="$2"
  if grep -Fq -- "$needle" "$WORKFLOW"; then
    fail "$description (unexpected: $needle)"
  else
    pass "$description"
  fi
}

assert_config_contains() {
  local needle="$1" description="$2"
  if grep -Fq -- "$needle" "$ACTIONLINT_CONFIG"; then
    pass "$description"
  else
    fail "$description (missing: $needle)"
  fi
}

if [ -f "$WORKFLOW" ]; then
  pass "Jetson CUDA evidence workflow exists"
else
  fail "Jetson CUDA evidence workflow is absent: $WORKFLOW"
  printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
  exit 1
fi

assert_contains '  workflow_dispatch:' "workflow is manually dispatched"
assert_contains '      candidate_sha:' "dispatch requires an explicit candidate SHA"
assert_absent '  push:' "workflow is not push-triggered"
assert_absent '  pull_request:' "workflow is not pull-request-triggered"
assert_absent '  schedule:' "workflow is not schedule-triggered"
assert_contains '  contents: read' "workflow grants only read-only repository contents"
assert_contains '  cancel-in-progress: false' "Jetson evidence lock never cancels an in-flight run"
assert_contains 'runs-on: [self-hosted, Linux, ARM64, jetson, aarch64]' "job routes to the registered Jetson labels exactly"
assert_config_contains 'labels: [gpu, cuda-12, jetson, aarch64]' "actionlint knows the registered custom Jetson labels"
assert_contains 'timeout-minutes: 90' "evidence job has a bounded timeout"
assert_contains 'persist-credentials: false' "checkout leaves no GitHub credential in git config"
assert_contains "ref: \${{ needs.validate-candidate.outputs.candidate_sha }}" "checkout uses the validated immutable candidate SHA"
assert_contains 'fetch-depth: 1' "checkout needs no mutable branch history"
assert_contains 'candidate SHA must equal the dispatched workflow commit' "candidate identity fails closed before checkout"
assert_contains 'git rev-parse HEAD' "checked-out source identity is retained"
assert_contains 'checkout must be clean after checkout' "dirty checkout fails before any evidence build"
assert_contains 'if [ -n "$checkout_status" ]; then' "checkout status is enforced before evidence"
assert_contains 'printf '\''clean\n'\'' > "$EVIDENCE_DIR/checkout-status.txt"' "clean checkout status is retained as evidence"
assert_contains 'uname -s' "host preflight checks Linux explicitly"
assert_contains 'uname -m' "host preflight checks AArch64 explicitly"
assert_contains 'nvidia,tegra' "host preflight requires the Tegra-family signal"
assert_contains 'nvgpu' "host preflight requires the classic Tegra CUDA signal"
assert_contains 'timeout 2s' "GPU probe has the ruled bounded timeout"
assert_contains 'build-python-cuda-tegra.sh' "workflow uses the existing host-native Tegra wheel wrapper"
assert_contains 'FATHOMDB_EMBED_DEVICE=cpu' "installed-wheel CPU policy smoke is retained"
assert_contains 'FATHOMDB_EMBED_DEVICE=auto' "installed-wheel auto policy smoke is retained"
assert_contains 'FATHOMDB_EMBED_DEVICE=cuda:0' "installed-wheel forced-CUDA policy smoke is retained"
assert_contains 'FATHOMDB_GPU_ALLOCATION_WITNESS=1' "forced CUDA smoke requires an in-process allocation witness"
assert_contains 'verify-tegra-gpu-witness.py' "workflow validates the retained Tegra witness"
assert_contains 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a' "evidence artifact uploader is pinned"
assert_contains 'retention-days: 30' "logs, wheel, and validated evidence have bounded retention"
assert_contains 'if: always()' "evidence artifact is retained after failure"
assert_absent 'twine upload' "workflow cannot publish Python artifacts"
assert_absent 'npm publish' "workflow cannot publish npm artifacts"
assert_absent 'cargo publish' "workflow cannot publish Rust artifacts"
assert_absent 'gh release' "workflow cannot create a GitHub release"

if [ "${JETSON_TEGRA_CI_FIXTURE:-0}" != "1" ]; then
  TMPROOT="$(mktemp -d)"
  cleanup() {
    case "$TMPROOT" in
      "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$TMPROOT" ;;
      *) printf 'refusing to remove unexpected temp path: %s\n' "$TMPROOT" >&2 ;;
    esac
  }
  trap cleanup EXIT

  MUTATED="$TMPROOT/jetson-tegra-cuda-evidence.yml"
  sed 's/cancel-in-progress: false/cancel-in-progress: true/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(JETSON_TEGRA_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'Jetson evidence lock never cancels an in-flight run' <<<"$mutation_out"; then
    pass "mutation proves the non-cancelling lock is load-bearing"
  else
    fail "lock mutation did not fail its assertion: $mutation_out"
  fi

  sed 's/FATHOMDB_GPU_ALLOCATION_WITNESS=1/FATHOMDB_GPU_ALLOCATION_WITNESS=0/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(JETSON_TEGRA_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'forced CUDA smoke requires an in-process allocation witness' <<<"$mutation_out"; then
    pass "mutation proves witness opt-in is load-bearing"
  else
    fail "witness mutation did not fail its assertion: $mutation_out"
  fi

  sed 's/if \[ -n "\$checkout_status" \]; then/if false; then/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(JETSON_TEGRA_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'checkout status is enforced before evidence' <<<"$mutation_out"; then
    pass "mutation proves the clean-checkout guard is load-bearing"
  else
    fail "clean-checkout mutation did not fail its assertion: $mutation_out"
  fi
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
