#!/usr/bin/env bash
# The security scanner remains observable, but its absence must not prevent a
# local release verification. Other pinned developer tools still gate normally.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

output="$TMPROOT/dev-environment-tools.out"
if GITLEAKS_BIN="$TMPROOT/missing-gitleaks" \
  bash "$REPO_ROOT/scripts/tests/test_dev_environment_tools.sh" >"$output" 2>&1; then
  :
else
  cat "$output" >&2
  echo 'FAIL  missing Gitleaks must be advisory' >&2
  exit 1
fi

if ! rg -Fq 'WARN  gitleaks:' "$output"; then
  cat "$output" >&2
  echo 'FAIL  missing Gitleaks warning is retained' >&2
  exit 1
fi

if ! rg -Fq 'dev-environment-tools: required pinned tools present' "$output"; then
  cat "$output" >&2
  echo 'FAIL  required-tool success summary is retained' >&2
  exit 1
fi

echo 'PASS  missing Gitleaks is report-only'
