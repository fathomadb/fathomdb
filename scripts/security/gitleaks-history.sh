#!/usr/bin/env bash
# Independently scan every reachable commit in CI with redacted findings.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/gitleaks-version.sh
. "$SCRIPT_DIR/../lib/gitleaks-version.sh"

repo="${1:-.}"
repo="$(git -C "$repo" rev-parse --show-toplevel)" || exit 1
gitleaks_bin="$(require_gitleaks_bin gitleaks-history)" || exit 1

exec "$gitleaks_bin" git --log-opts="--all" --redact=100 --no-banner --no-color --exit-code 1 "$repo"
