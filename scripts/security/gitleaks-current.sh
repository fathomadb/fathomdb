#!/usr/bin/env bash
# Scan the tracked working tree before any historical exception can be applied.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/gitleaks-version.sh
. "$SCRIPT_DIR/../lib/gitleaks-version.sh"

repo="${1:-.}"
repo="$(git -C "$repo" rev-parse --show-toplevel)" || exit 1
gitleaks_bin="$(require_gitleaks_bin gitleaks-current)" || exit 1

scan_root="$(mktemp -d)"
trap 'rm -rf "$scan_root"' EXIT

# `gitleaks dir` respects ignore files. Archive only tracked working-tree files
# so CI and local use both cover tracked source. Gitleaks excludes tracked log
# files by default, so scan those explicitly below as well.
git -C "$repo" ls-files -z | (cd "$repo" && tar --null --files-from=- -cf -) | tar -xf - -C "$scan_root"

cd "$scan_root"
set +e
"$gitleaks_bin" dir \
  --config "$SCRIPT_DIR/gitleaks-current.toml" \
  --ignore-gitleaks-allow \
  --redact=100 \
  --no-banner \
  --no-color \
  --exit-code 1 \
  --report-format template \
  --report-template "$SCRIPT_DIR/gitleaks-safe-report.tmpl" \
  --report-path - \
  .
scan_rc=$?

while IFS= read -r -d '' path; do
  git -C "$repo" show ":$path" |
    "$gitleaks_bin" stdin \
      --config "$SCRIPT_DIR/gitleaks-current.toml" \
      --ignore-gitleaks-allow \
      --redact=100 \
      --no-banner \
      --no-color \
      --exit-code 1 \
      --report-format template \
      --report-template "$SCRIPT_DIR/gitleaks-safe-report.tmpl" \
      --report-path - 2>/dev/null |
    awk -F'|' -v path="$path" 'NF == 3 { print $1 "|" path "|" $3 }'
  log_rc=${PIPESTATUS[1]}
  if [ "$log_rc" -ne 0 ]; then
    scan_rc=1
  fi
done < <(git -C "$repo" ls-files -z -- '*.log')
set -e

exit "$scan_rc"
