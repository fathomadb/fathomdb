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
assert_contains '      candidate_version:' "dispatch requires an explicit candidate release version"
assert_absent '  push:' "workflow is not push-triggered"
assert_absent '  pull_request:' "workflow is not pull-request-triggered"
assert_absent '  schedule:' "workflow is not schedule-triggered"
assert_contains '  contents: read' "workflow grants only read-only repository contents"
assert_contains '  cancel-in-progress: false' "Jetson evidence lock never cancels an in-flight run"
assert_contains 'runs-on: [self-hosted, Linux, ARM64, jetson, aarch64]' "job routes to the registered Jetson labels exactly"
assert_config_contains 'labels: [gpu, cuda-12, jetson, aarch64]' "actionlint knows the registered custom Jetson labels"
assert_contains 'timeout-minutes: 180' "evidence job has a bounded timeout"
assert_contains 'persist-credentials: false' "checkout leaves no GitHub credential in git config"
assert_contains "startsWith(github.ref, 'refs/heads/release/')" "self-hosted jobs are restricted to release branches"
assert_absent "refs/heads/release/0.8.24" "stale fixed release-branch predicates are absent"
assert_contains "ref: \${{ github.sha }}" "checkout uses GitHub's immutable dispatch SHA"
assert_absent "ref: \${{ needs.validate-candidate.outputs.candidate_sha }}" "dispatch input cannot select executed checkout bytes"
assert_contains 'fetch-depth: 1' "checkout needs no mutable branch history"
assert_contains 'candidate SHA must equal the dispatched workflow commit' "candidate identity fails closed before checkout"
assert_contains 'git rev-parse HEAD' "checked-out source identity is retained"
assert_contains 'checkout must be clean after checkout' "dirty checkout fails before any evidence build"
assert_contains 'if [ -n "$checkout_status" ]; then' "checkout status is enforced before evidence"
assert_contains 'printf '\''clean\n'\'' > "$EVIDENCE_DIR/checkout-status.txt"' "clean checkout status is retained as evidence"
assert_contains 'candidate version must match the dispatched release branch' "candidate version is bound to the release branch"
assert_contains '--base-version "$CANDIDATE_VERSION"' "Jetson builder receives the validated candidate version"
if awk '
  /name: Require the candidate version and release branch/ { in_step = 1; seen = 1; next }
  in_step && /^  [A-Za-z0-9_-]+:$/ { in_step = 0 }
  in_step && /^[[:space:]]+- name:/ { in_step = 0 }
  in_step && /^[[:space:]]+if:/ { conditional = 1 }
  END { exit(!seen || conditional) }
' "$WORKFLOW"; then
  pass "candidate-version check applies to every Jetson evidence run"
else
  fail "candidate-version check must not be conditional on publication"
fi
if awk '
  /name: Require the candidate version and release branch/ { in_step = 1; seen = 1; next }
  in_step && /^  [A-Za-z0-9_-]+:$/ { in_step = 0 }
  in_step && /^[[:space:]]+- name:/ { in_step = 0 }
  in_step && /^[[:space:]]+run: \|[[:space:]]*$/ { literal_run = 1 }
  in_step && /set -euo pipefail/ { strict_shell = 1 }
  in_step && /candidate version must match the dispatched release branch/ { version_argument = 1 }
  END { exit(!seen || !literal_run || !strict_shell || !version_argument) }
' "$WORKFLOW"; then
  pass "candidate-version command uses a strict literal block"
else
  fail "candidate-version command must use a strict literal block"
fi
assert_contains 'uname -s' "host preflight checks Linux explicitly"
assert_contains 'uname -m' "host preflight checks AArch64 explicitly"
assert_contains 'nvidia,tegra' "host preflight requires the Tegra-family signal"
assert_contains 'nvgpu' "host preflight requires the classic Tegra CUDA signal"
assert_contains 'timeout 2s' "GPU probe has the ruled bounded timeout"
assert_contains 'if [ ! -x "$HOME/.cargo/bin/cargo" ]; then' "workflow fails closed when the runner Cargo toolchain is absent"
assert_contains '"$HOME/.cargo/bin" >> "$GITHUB_PATH"' "workflow exposes the runner Cargo toolchain to every build step"
assert_contains 'build-python-cuda-tegra.sh' "workflow uses the existing host-native Tegra wheel wrapper"
assert_contains "fathomdb-*-linux_aarch64.whl" "post-build discovery accepts maturin's Linux AArch64 wheel tag"
assert_contains 'FATHOMDB_EMBED_DEVICE=cpu' "installed-wheel CPU policy smoke is retained"
assert_contains 'FATHOMDB_EMBED_DEVICE=auto' "installed-wheel auto policy smoke is retained"
assert_contains 'FATHOMDB_EMBED_DEVICE=cuda:0' "installed-wheel forced-CUDA policy smoke is retained"
assert_contains 'FATHOMDB_GPU_ALLOCATION_WITNESS=1' "forced CUDA smoke requires an in-process allocation witness"
assert_contains 'verify-tegra-gpu-witness.py' "workflow validates the retained Tegra witness"
assert_contains 'stage-tegra-pages-payload.sh' "Jetson stages an allowlisted distribution payload"
assert_contains 'fathomdb-tegra-pages-payload-${{ github.run_id }}-${{ github.run_attempt }}' "only the staged distribution path is uploaded"
assert_absent 'path: ${{ runner.temp }}/fathomdb-jetson-tegra-evidence-' "bulk SQLite and log evidence cannot enter the upload"
assert_absent 'Upload driverless ARM64 import evidence' "driverless job does not upload copied packages or smoke data"
assert_absent 'Upload post-publication Jetson smoke evidence' "post-publication smoke does not upload databases or logs"
upload_count="$(awk 'index($0, "uses: ./.github/actions/upload-artifact-retry") { n++ } END { print n+0 }' "$WORKFLOW")"
if [ "$upload_count" -eq 1 ]; then
  pass "the workflow uploads exactly one allowlisted cross-job artifact"
else
  fail "the workflow must upload exactly one allowlisted cross-job artifact"
fi

# The same Jetson run builds, smokes and retains the Tegra Node tarballs
# beside the wheel (ADR-0.8.27-jetson-tegra-node-addon-distribution).
evidence_job="$(awk '
  $0 == "  tegra-cuda-evidence:" { in_job = 1; next }
  in_job && /^  [A-Za-z0-9_-]+:$/ { exit }
  in_job { print }
' "$WORKFLOW")"
assert_job() {
  local needle="$1" description="$2"
  if grep -Fq -- "$needle" <<<"$evidence_job"; then
    pass "$description"
  else
    fail "$description (missing from tegra-cuda-evidence: $needle)"
  fi
}
assert_job_absent() {
  local needle="$1" description="$2"
  if grep -Fq -- "$needle" <<<"$evidence_job"; then
    fail "$description (unexpected in tegra-cuda-evidence: $needle)"
  else
    pass "$description"
  fi
}
assert_job 'actions/setup-node@48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e' "Jetson job provisions Node through the pinned setup action"
assert_job 'node-version: "25.9.0"' "Jetson job provisions the contract Node version"
assert_job 'package-manager-cache: false' "Jetson job does not restore a package-manager cache"
assert_job_absent 'registry-url' "Jetson job configures no npm registry authentication"
assert_job 'CUDA_TEGRA_NAPI_NPM_VERSION' "Jetson job asserts the contract npm version"
assert_job 'npm ci --ignore-scripts' "Jetson job installs src/ts dependencies without lifecycle scripts"
assert_job 'working-directory: src/ts' "Jetson job installs the src/ts lockfile"
assert_job 'build-napi-cuda-tegra.sh' "Jetson job uses the host-native Tegra N-API wrapper"
assert_job 'build-napi-cuda-tegra.sh --base-version "$CANDIDATE_VERSION" --assert-only' "Jetson job runs the N-API wrapper's assertion arm"
assert_job '--out "$EVIDENCE_DIR/node"' "Jetson job retains the packed Node tarballs in its evidence"
assert_job 'smoke-tegra-node-package.sh' "Jetson job smokes the installed Node packages"
assert_job '--source local' "Jetson Node smoke installs this run's packed tarballs"
assert_job '--candidate-sha "$CANDIDATE_SHA"' "Jetson Node smoke binds the validated candidate"
assert_job '--evidence-dir "$EVIDENCE_DIR/node-local-smoke"' "Jetson Node smoke writes local proof before payload staging"
assert_job_absent 'permissions:' "Jetson job keeps the workflow's read-only contents permission"
assert_job_absent ': write' "Jetson job holds no write permission"
assert_job_absent 'NPM_TOKEN' "Jetson job holds no npm credential"
assert_absent 'twine upload' "workflow cannot publish Python artifacts"
assert_absent 'npm publish' "workflow cannot publish npm artifacts"
assert_absent 'cargo publish' "workflow cannot publish Rust artifacts"
assert_absent 'gh release' "workflow cannot create a GitHub release"

# A hosted, non-Tegra ARM64 runner without a CUDA driver installs and imports
# this run's Tegra tarball and proves the loader ignores it there; Pages
# publication waits for it (ADR-0.8.27-jetson-tegra-node-addon-distribution,
# decisions 5 and 6).
job_block() {
  awk -v header="  $1:" '
    $0 == header { in_job = 1; next }
    in_job && /^  [A-Za-z0-9_-]+:$/ { exit }
    in_job { print }
  ' "$WORKFLOW"
}
driverless_job="$(job_block tegra-node-driverless-arm64)"
pages_job="$(job_block prepare-tegra-pages)"
assert_driverless() {
  local needle="$1" description="$2"
  if grep -Fq -- "$needle" <<<"$driverless_job"; then
    pass "$description"
  else
    fail "$description (missing from tegra-node-driverless-arm64: $needle)"
  fi
}
assert_driverless_absent() {
  local needle="$1" description="$2"
  if [ -n "$driverless_job" ] && ! grep -Fq -- "$needle" <<<"$driverless_job"; then
    pass "$description"
  else
    fail "$description (unexpected or job absent in tegra-node-driverless-arm64: $needle)"
  fi
}
assert_driverless 'runs-on: ubuntu-24.04-arm' "driverless import runs on a hosted ARM64 runner"
assert_driverless_absent 'self-hosted' "driverless import never runs on the Jetson"
assert_driverless 'needs: [validate-candidate, tegra-cuda-evidence]' "driverless import consumes this run's Jetson build"
assert_driverless "if: startsWith(github.ref, 'refs/heads/release/')" "driverless import is restricted to release branches"
assert_driverless $'    permissions:\n      contents: read' "driverless import holds only read-only contents"
assert_driverless_absent ': write' "driverless import holds no write permission"
assert_driverless 'timeout-minutes:' "driverless import has a bounded timeout"
assert_driverless 'persist-credentials: false' "driverless checkout leaves no credential"
assert_driverless "ref: \${{ github.sha }}" "driverless checkout uses the immutable dispatch SHA"
assert_driverless 'name: jetson-tegra-pages-payload-${{ github.run_id }}-${{ github.run_attempt }}' "driverless import downloads only this run's package payload"
assert_driverless 'SHA256SUMS' "driverless import checks the tarballs against the Jetson build digests"
assert_driverless 'smoke-tegra-node-driverless.sh' "driverless import runs the driverless smoke"
assert_driverless '--candidate-sha "$CANDIDATE_SHA"' "driverless smoke binds the validated candidate"
assert_driverless 'fathomdb-linux-arm64-gnu-tegra-$CANDIDATE_VERSION.tgz' "driverless smoke imports this run's Tegra tarball"
assert_driverless_absent 'uses: ./.github/actions/upload-artifact-retry' "driverless job uploads no redundant evidence"
assert_driverless_absent 'registry-url' "driverless import configures no npm registry authentication"
if grep -Fq 'needs: [tegra-cuda-evidence, tegra-node-driverless-arm64]' <<<"$pages_job"; then
  pass "Pages publication waits for the driverless ARM64 import"
else
  fail "Pages publication must need tegra-node-driverless-arm64"
fi

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

  sed 's/npm ci --ignore-scripts/npm ci/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(JETSON_TEGRA_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'Jetson job installs src/ts dependencies without lifecycle scripts' <<<"$mutation_out"; then
    pass "mutation proves disabled lifecycle scripts are load-bearing"
  else
    fail "lifecycle-script mutation did not fail its assertion: $mutation_out"
  fi

  sed 's/smoke-tegra-node-package\.sh/missing-node-smoke.sh/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(JETSON_TEGRA_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'Jetson job smokes the installed Node packages' <<<"$mutation_out"; then
    pass "mutation proves the installed Node smoke is load-bearing"
  else
    fail "Node-smoke mutation did not fail its assertion: $mutation_out"
  fi

  sed 's/needs: \[tegra-cuda-evidence, tegra-node-driverless-arm64\]/needs: tegra-cuda-evidence/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(JETSON_TEGRA_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'Pages publication must need tegra-node-driverless-arm64' <<<"$mutation_out"; then
    pass "mutation proves the driverless publication gate is load-bearing"
  else
    fail "driverless-gate mutation did not fail its assertion: $mutation_out"
  fi

  sed "s/startsWith(github.ref, 'refs\/heads\/release\/')/startsWith(github.ref, 'refs\/heads\/topic\/')/g" "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(JETSON_TEGRA_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] \
    && grep -Fq 'self-hosted jobs are restricted to release branches' <<<"$mutation_out"; then
    pass "mutation proves the Jetson release-branch contract is load-bearing"
  else
    fail "release-branch mutation did not fail its assertion: $mutation_out"
  fi
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
