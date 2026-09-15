#!/usr/bin/env bash
# Focused regression coverage for current architecture authority and navigation.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="${CHECKER_UNDER_TEST:-$REPO_ROOT/scripts/check-architecture-authority.py}"

TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

make_fixture() {
  local root="$1"
  rm -rf "$root"
  mkdir -p "$root/dev/design"
  cp "$REPO_ROOT/dev/README.md" "$root/dev/README.md"
  cp "$REPO_ROOT/dev/architecture.md" "$root/dev/architecture.md"
  cp "$REPO_ROOT/dev/design/README.md" "$root/dev/design/README.md"
  cp "$REPO_ROOT/dev/design/fathomdb-data-plane-architecture-v1.md" "$root/dev/design/"
  cp "$REPO_ROOT/dev/design/fathomdb-data-plane-architecture-v2.md" "$root/dev/design/"
}

expect_pass() {
  local root="$1" description="$2"
  if REPO_ROOT="$root" python3 "$CHECKER" >/dev/null; then
    printf 'PASS  %s\n' "$description"
  else
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
}

expect_fail() {
  local root="$1" description="$2"
  if REPO_ROOT="$root" python3 "$CHECKER" >/dev/null 2>&1; then
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
  printf 'PASS  %s\n' "$description"
}

assert_call_sites() {
  local agent_lint="$1" ci="$2"
  grep -Eq '^[[:space:]]*run_capped check-architecture-authority .*check-architecture-authority\.py' "$agent_lint" &&
    grep -Eq '^[[:space:]]*run: python3 scripts/check-architecture-authority\.py[[:space:]]*$' "$ci"
}

assert_test_registration() {
  local agent_test="$1"
  grep -Fqx \
    'run_tier_suite fast test-check-architecture-authority bash scripts/tests/test_check_architecture_authority.sh' \
    "$agent_test"
}

FIXTURE="$TMPROOT/fixture"
make_fixture "$FIXTURE"
expect_pass "$FIXTURE" 'one active architecture is linked from both indexes'

make_fixture "$FIXTURE"
sed -i 's/^status: SUPERSEDED$/status: ACTIVE/' "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'historical top-level snapshot cannot become active'

make_fixture "$FIXTURE"
sed -i '/^> \*\*Superseded architecture:/d' "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'historical snapshot requires the exact successor banner'

make_fixture "$FIXTURE"
sed -i 's#(design/fathomdb-data-plane-architecture-v2\.md)#(design/missing.md)#' \
  "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'historical banner must link the declared successor'

make_fixture "$FIXTURE"
sed -i 's#(design/fathomdb-data-plane-architecture-v2\.md)##' \
  "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'plain banner text is not a successor link'

make_fixture "$FIXTURE"
sed -i 's#^superseded_by: .*#superseded_by: dev/design/missing.md#' "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'historical successor path must resolve'

make_fixture "$FIXTURE"
sed -i 's#^superseded_by: .*#superseded_by:#' "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'blank required successor metadata fails closed'

make_fixture "$FIXTURE"
sed -i '0,/^status: ACTIVE$/s//status: SUPERSEDED/' "$FIXTURE/dev/design/fathomdb-data-plane-architecture-v2.md"
expect_fail "$FIXTURE" 'named successor must be active'

make_fixture "$FIXTURE"
cp "$FIXTURE/dev/design/fathomdb-data-plane-architecture-v1.md" \
  "$FIXTURE/dev/design/fathomdb-data-plane-architecture-v3.md"
sed -i '0,/^status: SUPERSEDED$/s//status: ACTIVE/' \
  "$FIXTURE/dev/design/fathomdb-data-plane-architecture-v3.md"
expect_fail "$FIXTURE" 'two active versioned architectures fail closed'

make_fixture "$FIXTURE"
sed -i 's#(design/fathomdb-data-plane-architecture-v2\.md)#(design/missing.md)#' \
  "$FIXTURE/dev/README.md"
expect_fail "$FIXTURE" 'dev index must link the correct active architecture target'

make_fixture "$FIXTURE"
sed -i 's#(design/fathomdb-data-plane-architecture-v2\.md)##g' \
  "$FIXTURE/dev/README.md"
expect_fail "$FIXTURE" 'plain dev-index text is not an architecture link'

make_fixture "$FIXTURE"
sed -i 's#(fathomdb-data-plane-architecture-v2\.md)#(missing.md)#' \
  "$FIXTURE/dev/design/README.md"
expect_fail "$FIXTURE" 'design index must link the correct active architecture target'

make_fixture "$FIXTURE"
sed -i 's/^status: SUPERSEDED$/metadata:\n  status: SUPERSEDED/' \
  "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'nested front-matter status cannot replace the required top-level key'

make_fixture "$FIXTURE"
sed -i '/^status: SUPERSEDED$/i status: ACTIVE' "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'duplicate required front-matter keys fail closed'

cp "$REPO_ROOT/scripts/agent-lint-md.sh" "$TMPROOT/agent-lint-md.sh"
cp "$REPO_ROOT/.github/workflows/ci.yml" "$TMPROOT/ci.yml"
cp "$REPO_ROOT/scripts/agent-test.sh" "$TMPROOT/agent-test.sh"
assert_call_sites "$TMPROOT/agent-lint-md.sh" "$TMPROOT/ci.yml" || {
  printf 'FAIL  local and docs-only CI must invoke the architecture checker\n' >&2
  exit 1
}
printf 'PASS  local and docs-only CI invoke the architecture checker\n'

sed -i 's/^\([[:space:]]*run: python3 scripts\/check-architecture-authority\.py\)/# \1/' \
  "$TMPROOT/ci.yml"
if assert_call_sites "$TMPROOT/agent-lint-md.sh" "$TMPROOT/ci.yml"; then
  printf 'FAIL  source-contract control accepted a commented CI invocation\n' >&2
  exit 1
fi
printf 'PASS  source-contract control rejects a commented CI invocation\n'

assert_test_registration "$TMPROOT/agent-test.sh" || {
  printf 'FAIL  normal fast tests must register the architecture regression suite\n' >&2
  exit 1
}
printf 'PASS  normal fast tests register the architecture regression suite\n'

sed -i '/^run_tier_suite fast test-check-architecture-authority /s/^/# /' \
  "$TMPROOT/agent-test.sh"
if assert_test_registration "$TMPROOT/agent-test.sh"; then
  printf 'FAIL  registration control accepted a commented test entry\n' >&2
  exit 1
fi
printf 'PASS  registration control rejects a commented test entry\n'

printf '\nAll architecture-authority tests passed\n'
