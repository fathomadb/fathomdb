#!/usr/bin/env bash
# Regression coverage for advisory warnings about GitHub Actions' built-in
# commit-message suppression annotations. These warnings must be visible at
# message creation, again before push, and for the PR title that becomes the
# configured squash-commit title. They must never block a commit or push.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WARNER="$REPO_ROOT/scripts/warn-ci-skip-message.sh"
COMMIT_MSG_HOOK="$REPO_ROOT/scripts/hooks/commit-msg"
PRE_PUSH_HOOK="$REPO_ROOT/scripts/hooks/pre-push"
INSTALL_HOOKS="$REPO_ROOT/scripts/install-hooks.sh"
CI_WORKFLOW="$REPO_ROOT/.github/workflows/ci.yml"
AGENT_TEST="$REPO_ROOT/scripts/agent-test.sh"

PASS=0
FAIL=0

pass() {
  printf 'PASS  %s\n' "$1"
  PASS=$((PASS + 1))
}

fail() {
  printf 'FAIL  %s\n' "$1" >&2
  FAIL=$((FAIL + 1))
}

WORK="$(mktemp -d)"
cleanup() {
  case "$WORK" in
    "${TMPDIR:-/tmp}"/*|/tmp/*) rm -rf "$WORK" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$WORK" >&2 ;;
  esac
}
trap cleanup EXIT
MESSAGE_FILE="$WORK/message"

expect_warning() {
  local message="$1" description="$2" out rc
  printf '%s\n' "$message" >"$MESSAGE_FILE"
  set +e
  out="$("$WARNER" --source fixture "$MESSAGE_FILE" 2>&1)"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ] && [[ "$out" == *"will suppress GitHub Actions"* ]] &&
    [[ "$out" == *"warning only"* ]]; then
    pass "$description"
  else
    fail "$description (rc=$rc, output=$out)"
  fi
}

expect_clean() {
  local message="$1" description="$2" out rc
  printf '%s\n' "$message" >"$MESSAGE_FILE"
  set +e
  out="$("$WARNER" --source fixture "$MESSAGE_FILE" 2>&1)"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ] && [ -z "$out" ]; then
    pass "$description"
  else
    fail "$description (rc=$rc, output=$out)"
  fi
}

if [ -x "$WARNER" ]; then
  pass "warning helper exists and is executable"

  tokens=(
    '[skip ci]'
    '[ci skip]'
    '[no ci]'
    '[skip actions]'
    '[actions skip]'
  )
  for token in "${tokens[@]}"; do
    expect_warning "checkpoint prose mentions $token unexpectedly" \
      "helper warns without blocking for $token"
  done
  expect_warning $'subject\n\ncontext\n\nskip-checks:true' \
    "helper warns without blocking for compact skip-checks trailer"
  expect_warning $'subject\n\ncontext\n\nskip-checks: true' \
    "helper warns without blocking for spaced skip-checks trailer"
  expect_clean "administrative change [ci-lite]" \
    "repository lite marker is not confused with GitHub suppression"
  expect_clean "skip_ci is descriptive prose, not a GitHub annotation" \
    "underscore lookalike does not warn"

  printf '%s\n' '[skip ci]' >"$MESSAGE_FILE"
  actions_out="$(GITHUB_ACTIONS=true "$WARNER" --source fixture "$MESSAGE_FILE" 2>&1)"
  if [[ "$actions_out" == '::warning title='* ]]; then
    pass "GitHub execution emits a workflow warning annotation"
  else
    fail "GitHub execution must emit a workflow warning annotation (output=$actions_out)"
  fi
else
  fail "warning helper exists and is executable"
fi

if [ -x "$COMMIT_MSG_HOOK" ] &&
  grep -Fq 'warn-ci-skip-message.sh' "$COMMIT_MSG_HOOK" &&
  grep -Fq '"$1"' "$COMMIT_MSG_HOOK"; then
  pass "tracked commit-msg hook checks the final proposed message"
else
  fail "tracked commit-msg hook must check the final proposed message"
fi

if grep -Fq 'warn-ci-skip-message.sh' "$PRE_PUSH_HOOK" &&
  grep -Fq 'git log --format=%B' "$PRE_PUSH_HOOK"; then
  pass "tracked pre-push hook rescans outgoing commit history"
else
  fail "tracked pre-push hook must rescan outgoing commit history"
fi

if grep -Fq 'scripts/hooks/commit-msg' "$INSTALL_HOOKS"; then
  pass "hook installer activates the tracked commit-msg hook"
else
  fail "hook installer must activate the tracked commit-msg hook"
fi

if grep -Fq 'github.event.pull_request.title' "$CI_WORKFLOW" &&
  grep -Fq 'warn-ci-skip-message.sh' "$CI_WORKFLOW"; then
  pass "changes job warns about the PR title used for squash landing"
else
  fail "changes job must warn about the PR title used for squash landing"
fi

if grep -Fq 'test-ci-skip-message-warning' "$AGENT_TEST"; then
  pass "warning regression fixture is registered in the fast test tier"
else
  fail "warning regression fixture must be registered in the fast test tier"
fi

printf '%s passed, %s failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
