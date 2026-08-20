#!/usr/bin/env bash
# Install the exact scanner version required by the local hook and CI guard.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/gitleaks-version.sh
. "$SCRIPT_DIR/lib/gitleaks-version.sh"

gitleaks_bin="$(find_gitleaks_bin || true)"
found_version=""
if [ -n "$gitleaks_bin" ] && [ -x "$gitleaks_bin" ]; then
  found_version="$(read_gitleaks_version "$gitleaks_bin" || true)"
fi

if [ "$found_version" != "$GITLEAKS_VERSION" ]; then
  if ! command -v go >/dev/null 2>&1; then
    printf 'gitleaks v%s is required but Go is unavailable; install Go and rerun scripts/install-gitleaks.sh.\n' \
      "$GITLEAKS_VERSION" >&2
    exit 1
  fi

  printf 'Installing gitleaks v%s via go install...\n' "$GITLEAKS_VERSION"
  GO111MODULE=on go install "$GITLEAKS_MODULE@v$GITLEAKS_VERSION"
  installed_bin="$(go env GOPATH)/bin/gitleaks"
  if [ ! -x "$installed_bin" ]; then
    printf 'gitleaks v%s installation did not produce %s.\n' "$GITLEAKS_VERSION" "$installed_bin" >&2
    exit 1
  fi
  mkdir -p "$HOME/.local/bin"
  install -m 0755 "$installed_bin" "$HOME/.local/bin/gitleaks"
fi

gitleaks_bin="$(require_gitleaks_bin install-gitleaks)" || exit 1
printf 'gitleaks v%s is installed at %s\n' "$GITLEAKS_VERSION" "$gitleaks_bin"

if [ -n "${GITHUB_PATH:-}" ]; then
  printf '%s\n' "$(dirname "$gitleaks_bin")" >>"$GITHUB_PATH"
fi
