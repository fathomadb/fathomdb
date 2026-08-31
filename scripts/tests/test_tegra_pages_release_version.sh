#!/usr/bin/env bash
# Slice 30: Pages publication is bound to the actual 0.8.24 project version.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="${TEGRA_PAGES_RELEASE_VERSION_CHECKER:-$REPO_ROOT/scripts/release/require-tegra-pages-release-version.sh}"
PASSED=0
FAILED=0

pass() { printf 'PASS  %s\n' "$1"; PASSED=$((PASSED + 1)); }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

TMPROOT="$(mktemp -d)"
cleanup() {
  case "$TMPROOT" in
    "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$TMPROOT" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$TMPROOT" >&2 ;;
  esac
}
trap cleanup EXIT

if [ -x "$CHECKER" ]; then
  pass "Tegra Pages release-version checker exists and is executable"
else
  fail "Tegra Pages release-version checker is absent or not executable: $CHECKER"
  printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
  exit 1
fi

GOOD="$TMPROOT/good-pyproject.toml"
BAD="$TMPROOT/bad-pyproject.toml"
printf '[project]\nname = "fathomdb"\nversion = "0.8.24"\n' > "$GOOD"
printf '[project]\nname = "fathomdb"\nversion = "0.8.23"\n' > "$BAD"

if "$CHECKER" --manifest "$GOOD" --expected-version 0.8.24; then
  pass "exact authorized base version passes"
else
  fail "exact authorized base version should pass"
fi

set +e
bad_out="$("$CHECKER" --manifest "$BAD" --expected-version 0.8.24 2>&1)"
bad_rc=$?
set -e
if [ "$bad_rc" -ne 0 ] && grep -Fq 'expected 0.8.24, got 0.8.23' <<<"$bad_out"; then
  pass "stale project metadata fails before Pages publication"
else
  fail "stale project metadata did not fail closed: $bad_out"
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
