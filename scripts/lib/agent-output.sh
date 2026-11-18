#!/usr/bin/env bash
# Shared output discipline for scripts/agent-*.sh.
#
# Output contract:
#   - Pass: silent on stdout (or "ok <verb> NNNms" if AGENT_VERBOSE=1).
#   - Fail: first 200 lines of captured output to stdout, then a footer
#           with the spill path and exit code. Spill file always written.
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
  "$@" >"$spill" 2>&1
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

  local total_lines
  # `wc -l | tr` would hide a failing wc behind tr's success; strip the
  # padding some wc implementations emit with parameter expansion instead.
  total_lines="$(wc -l <"$spill")"
  total_lines="${total_lines//[[:space:]]/}"

  printf 'FAIL %s (exit=%d, %s)\n' "$verb" "$rc" "$duration"
  printf -- '----\n'
  head -n 200 "$spill"
  if [ "$total_lines" -gt 200 ]; then
    printf -- '----\n'
    printf 'output truncated (%s lines total); full log: %s\n' "$total_lines" "$spill"
  else
    printf -- '----\n'
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
