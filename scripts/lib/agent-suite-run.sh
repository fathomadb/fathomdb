#!/usr/bin/env bash
# scripts/lib/agent-suite-run.sh — collect-all recording wrapper.
#
# Sourced by scripts/agent-test.sh ONLY. See
# scripts/tests/test_agent_test_collect_all.sh arm D for the static assertion
# that no other script in the repo sources this file.
#
# Registration is kept separate from execution so a cancellation can leave a
# complete plan and per-suite ledger in the job log. The shared run_capped
# helper retains its fail-fast exit contract for other tools.

set -u

_PLANNED_LABELS=()
_PLANNED_MODES=()
_PLANNED_ARG_COUNTS=()
_PLANNED_ARGS=()

_register_suite() {
  local mode="$1" label="$2"
  shift 2
  _PLANNED_LABELS+=("$label")
  _PLANNED_MODES+=("$mode")
  _PLANNED_ARG_COUNTS+=("$#")
  _PLANNED_ARGS+=("$@")
}

run_suite() {
  _register_suite run "$@"
}

skip_suite() {
  _register_suite skip "$@"
}

# Registration-order parallel arrays. Index i describes suite i.
_SUITE_LABELS=()
_SUITE_STATES=() # PASS | FAIL | SKIP | EXCL
_SUITE_RCS=()
_SUITE_MS=()

# Labels requested for exclusion (via exclude_suite), and which of those
# labels were actually matched by a real suite registration —
# used by suite_summary_and_exit to catch a typo'd --exclude-suite label
# that silently excluded nothing (a harness usage error, not a quiet no-op).
_EXCLUDED_LABELS=()
_EXCLUDED_MATCHED=()

# Usage: exclude_suite <label>
# Marks <label> before dispatch; unmatched exclusions are usage errors.
exclude_suite() {
  _EXCLUDED_LABELS+=("$1")
}

_is_excluded() {
  local label="$1" x
  for x in "${_EXCLUDED_LABELS[@]:-}"; do
    [ -n "$x" ] && [ "$x" = "$label" ] && return 0
  done
  return 1
}

_record() {
  local label="$1" state="$2" rc="$3" ms="$4"
  _SUITE_LABELS+=("$label")
  _SUITE_STATES+=("$state")
  _SUITE_RCS+=("$rc")
  _SUITE_MS+=("$ms")
  if agent_ci_live; then
    printf 'SUITE RESULT %s %s rc=%s duration_ms=%s\n' "$label" "$state" "$rc" "$ms"
  fi
}

# Escape data sent through GitHub's workflow-command protocol. Suite labels are
# currently static repository-owned strings, but this keeps a future label from
# opening a second command or annotation line.
_github_escape_command_data() {
  local value="$1"
  value="${value//%/%25}"
  value="${value//$'\r'/%0D}"
  value="${value//$'\n'/%0A}"
  printf '%s' "$value"
}

# Write a compact, deterministic result table to the GitHub run's front-page
# summary. It is intentionally emitted on success too: passing runs otherwise
# discard run_capped's per-suite timings, which made the slow tail impossible to
# measure. A missing summary path is a CI infrastructure failure, not a reason
# to silently drop observability.
_github_write_suite_summary() {
  local i n label state rc ms markdown_label

  [ -n "${GITHUB_ACTIONS:-}" ] || return 0
  if [ -z "${GITHUB_STEP_SUMMARY:-}" ]; then
    printf 'agent-test.sh: GITHUB_ACTIONS is set but GITHUB_STEP_SUMMARY is missing\n' >&2
    return 1
  fi

  n="${#_SUITE_LABELS[@]}"
  {
    printf '## FathomDB agent-test suite results\n\n'
    printf '| Suite | Status | Exit | Duration (ms) |\n'
    printf '| --- | --- | ---: | ---: |\n'
    for ((i = 0; i < n; i++)); do
      label="${_SUITE_LABELS[$i]}"
      state="${_SUITE_STATES[$i]}"
      rc="${_SUITE_RCS[$i]}"
      ms="${_SUITE_MS[$i]}"
      markdown_label="${label//|/\\|}"
      printf '| %s | %s | %s | %s |\n' "$markdown_label" "$state" "$rc" "$ms"
    done
    if [ "${#failed_labels[@]}" -gt 0 ]; then
      printf '\n### Failed suites\n\n'
      printf 'FAILED SUITES: %s\n' "${failed_labels[*]}"
    fi
  } >>"$GITHUB_STEP_SUMMARY"
}

_github_emit_failure_annotations() {
  local label escaped_label state i n duration

  [ -n "${GITHUB_ACTIONS:-}" ] || return 0
  n="${#_SUITE_LABELS[@]}"
  for ((i = 0; i < n; i++)); do
    state="${_SUITE_STATES[$i]}"
    [ "$state" = "FAIL" ] || continue
    label="${_SUITE_LABELS[$i]}"
    escaped_label="$(_github_escape_command_data "$label")"
    duration="$(agent_format_elapsed "${_SUITE_MS[$i]}")"
    printf '::error title=agent-test suite failed::%s (exit=%s, duration=%s)\n' \
      "$escaped_label" "${_SUITE_RCS[$i]}" "$duration"
  done
}

_execute_suite() {
  local mode="$1" label="$2"
  shift 2
  local _agent_capped_ms rc=0

  if _is_excluded "$label"; then
    _EXCLUDED_MATCHED+=("$label")
    _record "$label" EXCL "-" 0
    return
  fi
  if [ "$mode" = skip ]; then
    skip_notice "$label" "$@"
    _record "$label" SKIP 0 0
    return
  fi

  if agent_ci_live; then
    printf 'SUITE START %s\n' "$label"
  fi
  run_capped "$label" "$@" || rc=$?
  if [ "$rc" -eq 0 ]; then
    _record "$label" PASS "$rc" "$_agent_capped_ms"
  else
    _record "$label" FAIL "$rc" "$_agent_capped_ms"
  fi
}

_execute_plan() {
  local i count offset=0 label
  if agent_ci_live; then
    for label in "${_PLANNED_LABELS[@]}"; do
      printf 'SUITE PLAN %s\n' "$label"
    done
  fi
  for ((i = 0; i < ${#_PLANNED_LABELS[@]}; i++)); do
    count="${_PLANNED_ARG_COUNTS[$i]}"
    _execute_suite "${_PLANNED_MODES[$i]}" "${_PLANNED_LABELS[$i]}" "${_PLANNED_ARGS[@]:offset:count}"
    offset=$((offset + count))
  done
}

# Usage: suite_summary_and_exit
# Prints the deterministic collect-all summary in REGISTRATION ORDER and
# exits:
#   0 — zero FAILs
#   1 — at least one FAIL
#   2 — harness usage error (an --exclude-suite label matched no
#       registration at all — checked here, AFTER every run_suite/
#       skip_suite call site in the caller has executed, since that is the
#       first point at which "matched nothing" can be known).
suite_summary_and_exit() {
  local i n label state rc ms duration
  local registered=0 ran=0 passed=0 failed=0 skipped=0 excluded=0
  local failed_labels=()

  _execute_plan

  # Usage-error check FIRST: a requested exclusion that matched nothing
  # guards against a typo silently excluding nothing (falling through to a
  # normal, apparently-complete run).
  local x m matched
  for x in "${_EXCLUDED_LABELS[@]:-}"; do
    [ -z "$x" ] && continue
    matched=0
    for m in "${_EXCLUDED_MATCHED[@]:-}"; do
      if [ "$m" = "$x" ]; then
        matched=1
        break
      fi
    done
    if [ "$matched" -eq 0 ]; then
      printf 'agent-test.sh: usage error: --exclude-suite=%s matched no registered suite\n' "$x" >&2
      exit 2
    fi
  done

  n="${#_SUITE_LABELS[@]}"
  for ((i = 0; i < n; i++)); do
    state="${_SUITE_STATES[$i]}"
    registered=$((registered + 1))
    case "$state" in
      PASS)
        passed=$((passed + 1))
        ran=$((ran + 1))
        ;;
      FAIL)
        failed=$((failed + 1))
        ran=$((ran + 1))
        failed_labels+=("${_SUITE_LABELS[$i]}")
        ;;
      SKIP)
        skipped=$((skipped + 1))
        ;;
      EXCL)
        excluded=$((excluded + 1))
        ;;
    esac
  done

  if [ "$failed" -gt 0 ] || [ "${AGENT_VERBOSE:-0}" = "1" ]; then
    for ((i = 0; i < n; i++)); do
      label="${_SUITE_LABELS[$i]}"
      state="${_SUITE_STATES[$i]}"
      rc="${_SUITE_RCS[$i]}"
      ms="${_SUITE_MS[$i]}"
      duration="$(agent_format_elapsed "$ms")"
      printf '%s %s rc=%s %s\n' "$state" "$label" "$rc" "$duration"
    done
    printf 'registered=%d ran=%d passed=%d failed=%d skipped=%d excluded=%d\n' \
      "$registered" "$ran" "$passed" "$failed" "$skipped" "$excluded"
    if [ "$failed" -gt 0 ]; then
      printf 'FAILED SUITES: %s\n' "${failed_labels[*]}"
    fi
  else
    printf 'agent-test.sh: %d/%d suites passed (skipped=%d excluded=%d)\n' \
      "$passed" "$registered" "$skipped" "$excluded"
  fi

  if ! _github_write_suite_summary; then
    printf 'agent-test.sh: failed to write GitHub suite summary\n' >&2
    exit 1
  fi
  _github_emit_failure_annotations

  if [ "$failed" -gt 0 ]; then
    exit 1
  fi
  exit 0
}
