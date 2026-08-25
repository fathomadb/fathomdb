#!/usr/bin/env bash
# Slice 30: the Jetson builds without publication credentials; an explicit,
# hosted-only Pages publisher may expose the verified +tegra wheel.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKFLOW="${JETSON_TEGRA_CI_YML:-$REPO_ROOT/.github/workflows/jetson-tegra-cuda-evidence.yml}"
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

job_block() {
  local job_name="$1"
  awk -v job_name="$job_name" '
    $0 == "  " job_name ":" { in_job = 1; next }
    in_job && /^  [A-Za-z0-9_-]+:$/ { exit }
    in_job { print }
  ' "$WORKFLOW"
}

assert_job_contains() {
  local job_name="$1" needle="$2" description="$3"
  local block
  block="$(job_block "$job_name")"
  if grep -Fq -- "$needle" <<<"$block"; then
    pass "$description"
  else
    fail "$description (missing from $job_name: $needle)"
  fi
}

assert_job_absent() {
  local job_name="$1" needle="$2" description="$3"
  local block
  block="$(job_block "$job_name")"
  if grep -Fq -- "$needle" <<<"$block"; then
    fail "$description (unexpected in $job_name: $needle)"
  else
    pass "$description"
  fi
}

if [ -f "$WORKFLOW" ]; then
  pass "Jetson Tegra workflow exists"
else
  fail "Jetson Tegra workflow is absent: $WORKFLOW"
  printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
  exit 1
fi

assert_contains '      publish_to_pages:' "Pages publication needs an explicit dispatch input"
assert_contains '      default: false' "Pages publication defaults to disabled"
assert_contains '      type: boolean' "Pages publication input is boolean"
assert_contains '  prepare-tegra-pages:' "hosted Pages preparation job exists"
assert_contains '  deploy-tegra-pages:' "hosted Pages deployment job exists"
assert_contains 'runs-on: ubuntu-latest' "Pages work runs on a hosted runner"
assert_job_contains prepare-tegra-pages "github.ref == 'refs/heads/release/0.8.24'" "Pages preparation is bounded to the authorized release branch"
assert_job_contains deploy-tegra-pages "github.ref == 'refs/heads/release/0.8.24'" "Pages deployment is bounded to the authorized release branch"
assert_job_contains prepare-tegra-pages 'inputs.publish_to_pages == true' "Pages preparation requires explicit publication opt-in"
assert_job_contains deploy-tegra-pages 'inputs.publish_to_pages == true' "Pages deployment requires explicit publication opt-in"
assert_contains 'name: github-pages' "Pages deployment uses the dedicated GitHub environment"
assert_job_contains deploy-tegra-pages 'pages: write' "Pages deploy route requests Pages write access"
assert_job_contains deploy-tegra-pages 'id-token: write' "Pages deploy route requests its OIDC token"
assert_job_absent tegra-cuda-evidence 'pages: write' "Jetson evidence remains unable to deploy Pages"
assert_job_absent prepare-tegra-pages 'pages: write' "Pages preparation remains unable to deploy Pages"
assert_contains 'actions/download-artifact@' "hosted publisher consumes the Jetson artifact"
assert_contains 'jetson-tegra-cuda-evidence-${{ github.run_id }}-${{ github.run_attempt }}' "publisher accepts only this run's retained Jetson artifact"
assert_contains 'fathomdb-${expected_version}-*-linux_aarch64.whl' "publisher accepts only the honest Tegra wheel tag"
assert_contains 'bash scripts/release/build-tegra-pages-index.sh' "publisher delegates index construction to the exercised helper"
assert_contains '--version "$expected_version"' "publisher passes the exact +tegra version to the index helper"
assert_job_contains tegra-cuda-evidence 'require-tegra-pages-release-version.sh' "Jetson preflight rejects stale project metadata before a build"
assert_job_contains prepare-tegra-pages 'require-tegra-pages-release-version.sh' "publisher rejects stale project metadata before publication"
assert_contains '--expected-version 0.8.24' "publisher is hard-bound to the authorized base version"
assert_contains 'actions/upload-pages-artifact@' "publisher uploads a Pages deployment artifact"
assert_contains 'actions/deploy-pages@' "publisher deploys only through GitHub Pages"
assert_absent 'twine upload' "Pages publisher cannot upload to PyPI"
assert_absent 'npm publish' "Pages publisher cannot publish npm artifacts"
assert_absent 'cargo publish' "Pages publisher cannot publish Rust artifacts"
assert_absent 'CARGO_REGISTRY_TOKEN' "Pages publisher has no crates.io credential"
assert_absent 'NPM_TOKEN' "Pages publisher has no npm credential"

if [ "${TEGRA_PAGES_CI_FIXTURE:-0}" != "1" ]; then
  TMPROOT="$(mktemp -d)"
  cleanup() {
    case "$TMPROOT" in
      "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$TMPROOT" ;;
      *) printf 'refusing to remove unexpected temp path: %s\n' "$TMPROOT" >&2 ;;
    esac
  }
  trap cleanup EXIT

  MUTATED="$TMPROOT/jetson-tegra-cuda-evidence.yml"
  sed '0,/default: false/s//default: true/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(TEGRA_PAGES_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'Pages publication defaults to disabled' <<<"$mutation_out"; then
    pass "mutation proves publication remains opt-in"
  else
    fail "publication-default mutation did not fail its assertion: $mutation_out"
  fi

  sed '0,/inputs.publish_to_pages == true/! s/inputs.publish_to_pages == true/inputs.publish_to_pages == false/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(TEGRA_PAGES_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'Pages deployment requires explicit publication opt-in' <<<"$mutation_out"; then
    pass "mutation proves a Pages deploy cannot run without opt-in"
  else
    fail "publication-condition mutation did not fail its assertion: $mutation_out"
  fi

  sed '0,/build-tegra-pages-index\.sh/s//missing-tegra-pages-index.sh/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(TEGRA_PAGES_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'publisher delegates index construction to the exercised helper' <<<"$mutation_out"; then
    pass "mutation proves the publisher cannot bypass the exercised index helper"
  else
    fail "index-helper mutation did not fail its assertion: $mutation_out"
  fi

  sed '0,/require-tegra-pages-release-version\.sh/s//missing-tegra-pages-release-version.sh/' "$WORKFLOW" >"$MUTATED"
  set +e
  mutation_out="$(TEGRA_PAGES_CI_FIXTURE=1 JETSON_TEGRA_CI_YML="$MUTATED" bash "$0" 2>&1)"
  mutation_rc=$?
  set -e
  if [ "$mutation_rc" -ne 0 ] && grep -Fq 'Jetson preflight rejects stale project metadata before a build' <<<"$mutation_out"; then
    pass "mutation proves stale project metadata cannot reach publication"
  else
    fail "release-version-checker mutation did not fail its assertion: $mutation_out"
  fi
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
