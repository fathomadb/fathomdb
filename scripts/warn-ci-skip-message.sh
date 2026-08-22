#!/usr/bin/env bash
# Warn when text contains a GitHub Actions built-in workflow-suppression
# annotation. Advisory only: callers must not turn this warning into a gate.
set -euo pipefail

source_label="message"
if [ "${1:-}" = "--source" ]; then
  if [ "$#" -lt 2 ]; then
    printf 'usage: %s [--source LABEL] [MESSAGE_FILE|-]\n' "$0" >&2
    exit 0
  fi
  source_label="$2"
  shift 2
fi

message_file="${1:--}"
if [ "$message_file" = "-" ]; then
  message="$(</dev/stdin)"
elif [ -r "$message_file" ]; then
  message="$(<"$message_file")"
else
  printf 'WARNING: cannot inspect %s at %s; warning only, operation continues.\n' \
    "$source_label" "$message_file" >&2
  exit 0
fi

suppresses=false
case "$message" in
  *'[skip ci]'*|*'[ci skip]'*|*'[no ci]'*|*'[skip actions]'*|*'[actions skip]'*)
    suppresses=true
    ;;
esac

if [ "$suppresses" = false ]; then
  while IFS= read -r line; do
    case "$line" in
      'skip-checks:true'|'skip-checks: true')
        suppresses=true
        break
        ;;
    esac
  done <<<"$message"
fi

if [ "$suppresses" = true ]; then
  detail="$source_label contains a built-in suppression annotation; if it is a PR HEAD message or enters the squash commit, it will suppress GitHub Actions push/pull_request workflows. This is a warning only; the operation continues. Use the repository's exact-line [ci-lite] marker when proportional CI is intended."
  if [ "${GITHUB_ACTIONS:-false}" = "true" ]; then
    printf '::warning title=GitHub Actions workflow suppression::%s\n' "$detail"
  else
    printf 'WARNING: %s\n' "$detail" >&2
  fi
fi

exit 0
