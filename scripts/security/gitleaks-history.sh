#!/usr/bin/env bash
# Independently classify every reachable commit after the current-tree scan.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/gitleaks-version.sh
. "$SCRIPT_DIR/../lib/gitleaks-version.sh"

repo="${1:-.}"
repo="$(git -C "$repo" rev-parse --show-toplevel)" || exit 1
# Gitleaks gives inherited configuration priority over the repository policy.
# This boundary owns its policy, so callers cannot suppress historical findings.
unset GITLEAKS_CONFIG
unset GITLEAKS_CONFIG_TOML
"$SCRIPT_DIR/gitleaks-current.sh" "$repo"
gitleaks_bin="$(require_gitleaks_bin gitleaks-history)" || exit 1

scan_root="$(mktemp -d)"
trap 'rm -rf "$scan_root"' EXIT
safe_report="$scan_root/safe-report"
history_repo="$scan_root/history.git"
empty_ignore="$scan_root/empty-ignore"
: >"$empty_ignore"
if [ -s "$empty_ignore" ]; then
  printf '%s\n' 'gitleaks-history: owned ignore input must be empty' >&2
  exit 1
fi

if ! git clone --mirror "$repo" "$history_repo" >"$scan_root/mirror-output" 2>&1; then
  printf '%s\n' 'gitleaks-history: could not create isolated history scan source' >&2
  exit 1
fi

set +e
"$gitleaks_bin" git \
  --ignore-gitleaks-allow \
  --gitleaks-ignore-path "$empty_ignore" \
  --log-opts="--all" \
  --redact=100 \
  --no-banner \
  --no-color \
  --exit-code 1 \
  --report-format template \
  --report-template "$SCRIPT_DIR/gitleaks-history-safe-report.tmpl" \
  --report-path "$safe_report" \
  "$history_repo" >"$scan_root/scanner-output" 2>&1
scan_rc=$?
set -e

case "$scan_rc" in
  0 | 1) ;;
  *)
    printf '%s\n' 'gitleaks-history: scanner did not produce a comparable safe report' >&2
    exit 1
    ;;
esac

python3 "$SCRIPT_DIR/check-gitleaks-history.py" \
  "$SCRIPT_DIR/gitleaks-history-manifest.json" "$safe_report"
