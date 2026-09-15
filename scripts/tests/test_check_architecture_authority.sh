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
  grep -Fq 'check-architecture-authority.py' "$agent_lint" &&
    grep -Fq 'check-architecture-authority.py' "$ci"
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
sed -i 's#^superseded_by: .*#superseded_by: dev/design/missing.md#' "$FIXTURE/dev/architecture.md"
expect_fail "$FIXTURE" 'historical successor path must resolve'

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
sed -i '/fathomdb-data-plane-architecture-v2\.md/d' "$FIXTURE/dev/README.md"
expect_fail "$FIXTURE" 'dev index must link the active architecture'

make_fixture "$FIXTURE"
sed -i '/fathomdb-data-plane-architecture-v2\.md/d' "$FIXTURE/dev/design/README.md"
expect_fail "$FIXTURE" 'design index must link the active architecture'

cp "$REPO_ROOT/scripts/agent-lint-md.sh" "$TMPROOT/agent-lint-md.sh"
cp "$REPO_ROOT/.github/workflows/ci.yml" "$TMPROOT/ci.yml"
assert_call_sites "$TMPROOT/agent-lint-md.sh" "$TMPROOT/ci.yml" || {
  printf 'FAIL  local and docs-only CI must invoke the architecture checker\n' >&2
  exit 1
}
printf 'PASS  local and docs-only CI invoke the architecture checker\n'

sed -i '/check-architecture-authority\.py/d' "$TMPROOT/ci.yml"
if assert_call_sites "$TMPROOT/agent-lint-md.sh" "$TMPROOT/ci.yml"; then
  printf 'FAIL  source-contract control did not detect missing CI invocation\n' >&2
  exit 1
fi
printf 'PASS  source-contract control detects a missing invocation\n'

printf '\nAll architecture-authority tests passed\n'
