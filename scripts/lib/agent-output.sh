#!/usr/bin/env bash
# Shared output discipline for scripts/agent-*.sh.
#
# Output contract:
#   - Pass: silent on stdout (or "ok <verb> NNNms" if AGENT_VERBOSE=1).
#   - Fail: the exact exit and spill path. Locally print the first 200 lines.
#           GitHub receives an immediate bounded redacted first diagnostic.
#
# Rationale: keep agent context clean on the happy path; on failure give
# the agent the *start* of the diagnostic (where the actionable error
# lives) and a path to the rest. Cap is on output, not capture — the
# spill file always has the full log.

set -u

agent_monotonic_ms() {
  local value
  value="$(python3 -c 'import time; print(time.monotonic_ns() // 1_000_000)' 2>/dev/null)" || return 1
  [[ "$value" =~ ^[0-9]{1,16}$ ]] || return 1
  printf '%s\n' "$value"
}

agent_elapsed_ms() {
  local start_ms="$1" end_ms
  end_ms="$(agent_monotonic_ms)" || end_ms=""
  if [ -n "$start_ms" ] && [ -n "$end_ms" ] && ((10#$end_ms >= 10#$start_ms)); then
    printf '%d\n' "$((10#$end_ms - 10#$start_ms))"
  else
    printf 'unavailable\n'
  fi
}

agent_format_elapsed() {
  if [ "$1" = unavailable ]; then
    printf '%s' "$1"
  else
    printf '%sms' "$1"
  fi
}

agent_redact_log() {
  awk '
    /(^|[[:space:]"{,])"?(PATH|PYTHONPATH|NODE_PATH|LD_[[:alnum:]_]*|HOME|SSH_AUTH_SOCK|XDG_[[:alnum:]_]*|GITHUB_[[:alnum:]_]*|RUNNER_[[:alnum:]_]*|ACTIONS_[[:alnum:]_]*|[[:alnum:]_]*(TOKEN|SECRET|PASSWORD|KEY)[[:alnum:]_]*)"?[[:space:]]*[:=]/ {
      print "[REDACTED environment transcript line]"
      next
    }
    { print }
  ' "$1"
}

agent_ci_live() {
  [ "${GITHUB_ACTIONS:-}" = true ] && [ "${AGENT_CI_LIVE:-}" = 1 ]
}

_agent_live_failure() {
  local spill="$1" resume_token
  resume_token="$(python3 -c 'import secrets; print(secrets.token_hex(24))')" || return 1
  [[ "$resume_token" =~ ^[0-9a-f]{48}$ ]] || return 1
  printf '::stop-commands::%s\n' "$resume_token"
  local rc=0
  (
    set -o pipefail
    agent_redact_log "$spill" | python3 -c '
import sys

source = sys.stdin.buffer
output = sys.stdout.buffer
edge = 32 * 1024
first = source.read(edge)
last = source.read(edge)
omitted = False
while chunk := source.read(edge):
    omitted = True
    last = (last + chunk)[-edge:]
output.write(first)
if omitted:
    output.write(b"\n[live diagnostic truncated; showing the first and last 32 KiB]\n")
output.write(last)
'
  ) || rc=$?
  printf '\n::%s::\n' "$resume_token"
  return "$rc"
}

# Usage: run_capped <verb> <command...>
# Returns the underlying command's exit code.
run_capped() {
  local verb="$1"
  shift

  _agent_capped_ms=unavailable
  local spill_dir="${AGENT_LOG_DIR:-/tmp}"
  if [[ "$spill_dir" != /* ]] || [ ! -d "$spill_dir" ] || [ -L "$spill_dir" ]; then
    printf 'run_capped: spill directory must be an existing absolute regular directory: %s\n' "$spill_dir" >&2
    return 2
  fi
  local spill="$spill_dir/fathomdb-agent-${verb}-$$.log"
  local start_ms duration
  local rc=0

  start_ms="$(agent_monotonic_ms)" || start_ms=""

  # Capture combined stdout+stderr.
  set +e
  (unset AGENT_CI_LIVE; "$@") >"$spill" 2>&1
  rc=$?
  set -e
  set -u

  _agent_capped_ms="$(agent_elapsed_ms "$start_ms")"
  duration="$(agent_format_elapsed "$_agent_capped_ms")"

  if [ "$rc" -eq 0 ]; then
    if [ "${AGENT_VERBOSE:-0}" = "1" ]; then
      printf 'ok %s %s\n' "$verb" "$duration"
    fi
    rm -f "$spill"
    return 0
  fi

  printf 'FAIL %s (exit=%d, %s)\n' "$verb" "$rc" "$duration"
  if agent_ci_live; then
    if [ "${_agent_first_failure_emitted:-0}" -eq 0 ]; then
      _agent_first_failure_emitted=1
      if ! _agent_live_failure "$spill"; then
        printf 'Could not stream the redacted failure diagnostic.\n' >&2
      fi
    fi
    printf 'full log: %s\n' "$spill"
    return "$rc"
  fi

  local total_lines
  total_lines="$(wc -l <"$spill")"
  total_lines="${total_lines//[[:space:]]/}"
  printf -- '----\n'
  head -n 200 "$spill"
  printf -- '----\n'
  if [ "$total_lines" -gt 200 ]; then
    printf 'output truncated (%s lines total); full log: %s\n' "$total_lines" "$spill"
  else
    printf 'full log: %s\n' "$spill"
  fi
  return "$rc"
}

# Usage: skip_notice <verb> <reason>
# One-line skip notice. Returns 0.
skip_notice() {
  local verb="$1"; shift
  if [ "${AGENT_VERBOSE:-0}" = "1" ]; then
    printf 'skip %s: %s\n' "$verb" "$*"
  fi
  return 0
}

# Usage: cd_repo_root
cd_repo_root() {
  # `git rev-parse` failing here used to degrade to `cd ""` — a bash no-op
  # that leaves the caller running in an arbitrary cwd. Bind and check it.
  local _repo_toplevel
  _repo_toplevel="$(git rev-parse --show-toplevel)" || return 1
  cd "$_repo_toplevel" || return 1
}
