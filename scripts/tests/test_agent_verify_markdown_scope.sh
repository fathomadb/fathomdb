#!/usr/bin/env bash
# Regression guard for agent-verify's narrow Markdown verification scope.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERIFY="$REPO_ROOT/scripts/agent-verify.sh"

FAILED=0
pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

TMPROOT="$(mktemp -d)"
cleanup() {
  case "$TMPROOT" in
    "${TMPDIR:-/tmp}"/*|/tmp/*) rm -rf "$TMPROOT" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$TMPROOT" >&2 ;;
  esac
}
trap cleanup EXIT

FIXTURE="$TMPROOT/repo"
mkdir -p "$FIXTURE/scripts"
cp "$VERIFY" "$FIXTURE/scripts/agent-verify.sh"
git -C "$FIXTURE" init -q

make_stub() {
  local name="$1" marker="$2"
  {
    printf '#!/usr/bin/env bash\n'
    printf 'set -euo pipefail\n'
    # The generated stub expands this variable when it runs.
    # shellcheck disable=SC2016
    printf 'printf '\''%%s\\n'\'' %q >>"$VERIFY_SCOPE_LOG"\n' "$marker"
  } >"$FIXTURE/scripts/$name"
  chmod +x "$FIXTURE/scripts/$name"
}

make_stub agent-lint-md.sh markdown
make_stub agent-lint.sh lint
make_stub agent-typecheck.sh typecheck
make_stub agent-security.sh security
make_stub agent-test.sh test

export VERIFY_SCOPE_LOG="$TMPROOT/calls.log"

set +e
MARKDOWN_OUT="$(cd "$FIXTURE" && bash scripts/agent-verify.sh --scope=markdown 2>&1)"
MARKDOWN_RC=$?
set -e
MARKDOWN_CALLS="$(cat "$VERIFY_SCOPE_LOG" 2>/dev/null || true)"
if [ "$MARKDOWN_RC" -eq 0 ] && [ "$MARKDOWN_CALLS" = "markdown" ]; then
  pass "Markdown scope delegates only to agent-lint-md.sh"
else
  fail "Markdown scope must run only the Markdown gate: rc=$MARKDOWN_RC calls=$MARKDOWN_CALLS out=$MARKDOWN_OUT"
fi

if grep -qF 'scope=markdown' <<<"$MARKDOWN_OUT" \
  && grep -qF 'lint,typecheck,security,test skipped' <<<"$MARKDOWN_OUT"; then
  pass "Markdown scope reports its deliberately skipped broad gates"
else
  fail "Markdown scope must describe its narrow evidence: $MARKDOWN_OUT"
fi

printf '#!/usr/bin/env bash\nexit 7\n' >"$FIXTURE/scripts/agent-lint-md.sh"
set +e
MARKDOWN_FAIL_OUT="$(cd "$FIXTURE" && bash scripts/agent-verify.sh --scope=markdown 2>&1)"
MARKDOWN_FAIL_RC=$?
set -e
if [ "$MARKDOWN_FAIL_RC" -eq 1 ] && grep -qF 'FAIL verify at scope=markdown' <<<"$MARKDOWN_FAIL_OUT"; then
  pass "Markdown scope propagates a failing Markdown gate"
else
  fail "Markdown scope must fail when agent-lint-md.sh fails: rc=$MARKDOWN_FAIL_RC out=$MARKDOWN_FAIL_OUT"
fi

: >"$VERIFY_SCOPE_LOG"
set +e
DEFAULT_OUT="$(cd "$FIXTURE" && bash scripts/agent-verify.sh 2>&1)"
DEFAULT_RC=$?
set -e
DEFAULT_CALLS="$(cat "$VERIFY_SCOPE_LOG" 2>/dev/null || true)"
if [ "$DEFAULT_RC" -eq 0 ] && [ "$DEFAULT_CALLS" = $'lint\ntypecheck\nsecurity\ntest' ]; then
  pass "default verification remains the full lint/typecheck/security/test gate"
else
  fail "default verification contract changed: rc=$DEFAULT_RC calls=$DEFAULT_CALLS out=$DEFAULT_OUT"
fi

set +e
INVALID_OUT="$(cd "$FIXTURE" && bash scripts/agent-verify.sh --scope=source 2>&1)"
INVALID_RC=$?
MIXED_OUT="$(cd "$FIXTURE" && bash scripts/agent-verify.sh --scope=markdown --tier=fast 2>&1)"
MIXED_RC=$?
MIXED_REVERSED_OUT="$(cd "$FIXTURE" && bash scripts/agent-verify.sh --tier=fast --scope=markdown 2>&1)"
MIXED_REVERSED_RC=$?
set -e
if [ "$INVALID_RC" -eq 2 ] && grep -qF 'Usage:' <<<"$INVALID_OUT"; then
  pass "unknown verification scopes fail before any gate runs"
else
  fail "unknown scope must be a usage error: rc=$INVALID_RC out=$INVALID_OUT"
fi
if [ "$MIXED_RC" -eq 2 ] && grep -qF 'cannot combine' <<<"$MIXED_OUT" \
  && [ "$MIXED_REVERSED_RC" -eq 2 ] \
  && grep -qF 'cannot combine' <<<"$MIXED_REVERSED_OUT"; then
  pass "Markdown scope cannot be combined with a test tier in either order"
else
  fail "scope/tier ambiguity must be rejected: forward=$MIXED_RC/$MIXED_OUT reverse=$MIXED_REVERSED_RC/$MIXED_REVERSED_OUT"
fi

if [ "$FAILED" -ne 0 ]; then
  printf 'FAIL  %d agent-verify Markdown-scope assertion(s) failed\n' "$FAILED" >&2
  exit 1
fi
printf 'PASS  all agent-verify Markdown-scope assertions passed\n'
