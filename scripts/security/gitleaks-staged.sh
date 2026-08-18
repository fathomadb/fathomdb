#!/usr/bin/env bash
# Fail the commit before a staged credential reaches repository history.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/gitleaks-version.sh
. "$SCRIPT_DIR/../lib/gitleaks-version.sh"

repo="${1:-.}"
repo="$(git -C "$repo" rev-parse --show-toplevel)" || exit 1
gitleaks_bin="$(require_gitleaks_bin gitleaks-staged)" || exit 1

if git -C "$repo" diff --cached --quiet; then
  exit 0
fi

exec "$gitleaks_bin" git --staged \
  --config "$SCRIPT_DIR/gitleaks-current.toml" \
  --redact=100 --no-banner --no-color --exit-code 1 "$repo"
