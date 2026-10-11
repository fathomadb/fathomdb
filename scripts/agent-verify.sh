#!/usr/bin/env bash
# Full scope preflights checkout-local developer tools, then runs lint ->
# typecheck -> security -> test in latency order and
# short-circuits on first failure. Markdown scope delegates to agent-lint-md.sh.
# This is the agent-loop gate. The broader CI gate is scripts/check.sh.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# `git rev-parse` failing here used to degrade to `cd ""` — a bash no-op that
# leaves the script running in an arbitrary cwd. Bind and check it instead.
cd_repo_root() {
  local _repo_toplevel
  _repo_toplevel="$(git rev-parse --show-toplevel)" || return 1
  cd "$_repo_toplevel" || return 1
}
cd_repo_root

usage() {
  cat >&2 <<'USAGE'
Usage: agent-verify.sh [--tier=fast|heavy|all] | [--scope=markdown]

  --tier=fast|heavy|all  Select the corresponding agent-test tier. all is the
                         local default and preserves the full verifier.
  --scope=markdown       Run only the authoritative Markdown gate. This skips
                         broad lint, typecheck, security, and test gates.
USAGE
}

verify_tier="all"
verify_scope="full"
tier_selected=0
scope_selected=0
if [ "$#" -gt 2 ]; then
  usage
  exit 2
fi
while [ "$#" -gt 0 ]; do
  case "$1" in
    --tier=fast|--tier=heavy|--tier=all)
      if [ "$scope_selected" -eq 1 ]; then
        printf 'agent-verify.sh: cannot combine --scope=markdown with --tier\n' >&2
        usage
        exit 2
      fi
      if [ "$tier_selected" -eq 1 ]; then
        usage
        exit 2
      fi
      verify_tier="${1#--tier=}"
      tier_selected=1
      ;;
    --scope=markdown)
      if [ "$tier_selected" -eq 1 ]; then
        printf 'agent-verify.sh: cannot combine --scope=markdown with --tier\n' >&2
        usage
        exit 2
      fi
      if [ "$scope_selected" -eq 1 ]; then
        usage
        exit 2
      fi
      verify_scope="markdown"
      scope_selected=1
      ;;
    *)
      usage
      exit 2
      ;;
  esac
  shift
done

start=$(date +%s)

if [ "$verify_scope" = "markdown" ]; then
  printf 'verify scope=markdown: running agent-lint-md.sh only; lint,typecheck,security,test skipped\n'
  if ! "$SCRIPT_DIR/agent-lint-md.sh"; then
    end=$(date +%s)
    printf 'FAIL verify at scope=markdown (%ss elapsed)\n' "$((end - start))"
    exit 1
  fi
  if [ "${AGENT_VERBOSE:-0}" = "1" ]; then
    end=$(date +%s)
    printf 'ok verify scope=markdown %ss\n' "$((end - start))"
  fi
  exit 0
fi

verify_preflight() {
  local tool_output
  if [ ! -x .venv/bin/python ]; then
    printf 'FAIL verify preflight: checkout-owned .venv/bin/python is missing; prepare this checkout before full verification.\n' >&2
    return 1
  fi
  if ! .venv/bin/python -m pip --version >/dev/null 2>&1; then
    printf 'FAIL verify preflight: checkout-owned .venv/bin/python cannot run pip.\n' >&2
    return 1
  fi
  if ! tool_output="$(bash "$SCRIPT_DIR/tests/test_dev_environment_tools.sh" 2>&1)"; then
    printf '%s\n' "$tool_output" >&2
    return 1
  fi
}

if ! verify_preflight; then
  end=$(date +%s)
  printf 'FAIL verify at step=preflight (%ss elapsed)\n' "$((end - start))" >&2
  exit 1
fi

run_step() {
  local step="$1"
  if ! "$SCRIPT_DIR/agent-$step.sh"; then
    local end
    end=$(date +%s)
    printf 'FAIL verify at step=%s (%ss elapsed)\n' "$step" "$((end - start))"
    return 1
  fi
}

if [ "$verify_tier" != "heavy" ]; then
  run_step lint || exit 1
  run_step typecheck || exit 1
  # AC-036/037/038/050a/050c. STRICT=1 promotes toolchain blockers to
  # hard failures so the gate is real (rc=2 → exit). Local dev hosts need strace
  # (run scripts/bootstrap.sh) and a ptrace-capable executor for AC-036; rerun
  # unconfined when the sandbox denies ptrace rather than disabling the gate.
  #
  # AC037_LIVE_OPTIONAL=1: this gate runs on ubuntu-latest (and most dev hosts),
  # where unprivileged userns is blocked by AppArmor, so AC-037's LIVE netns
  # layer cannot run here (rc=3, environmental). The AUTHORITATIVE AC-037-live
  # gate is the dedicated ubuntu-22.04 `security` CI job (STRICT=1, no opt-in),
  # and the offline catch + policy self-test still run STRICT here. So we accept
  # the userns-unavailable downgrade for that ONE layer without failing verify —
  # while a real egress VIOLATION, a catch failure, or any other toolchain
  # BLOCKER still fails this gate. See scripts/security/lib-gate-policy.sh.
  STRICT=1 AC037_LIVE_OPTIONAL=1 bash "$SCRIPT_DIR/agent-security.sh" || exit 1
fi

if ! bash "$SCRIPT_DIR/agent-test.sh" "--tier=$verify_tier"; then
  end=$(date +%s)
  printf 'FAIL verify at step=test tier=%s (%ss elapsed)\n' "$verify_tier" "$((end - start))"
  exit 1
fi

end=$(date +%s)
if [ "${AGENT_VERBOSE:-0}" = "1" ]; then
  printf 'ok verify tier=%s %ss\n' "$verify_tier" "$((end - start))"
fi
