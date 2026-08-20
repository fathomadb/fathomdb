#!/usr/bin/env bash
# The scanner identifies its release in Go build metadata, rather than reliably
# through `gitleaks version`; the latter is unset in some release builds.
readonly GITLEAKS_MODULE="github.com/zricethezav/gitleaks/v8"
readonly GITLEAKS_VERSION="8.30.1"

read_gitleaks_version() {
  local gitleaks_bin="$1" build_metadata

  if ! command -v go >/dev/null 2>&1; then
    return 1
  fi

  build_metadata="$(go version -m "$gitleaks_bin" 2>/dev/null)" || return 1
  awk -v module="$GITLEAKS_MODULE" '$1 == "mod" && $2 == module { sub(/^v/, "", $3); print $3; exit }' \
    <<<"$build_metadata"
}

find_gitleaks_bin() {
  local gitleaks_bin

  if [ -n "${GITLEAKS_BIN:-}" ]; then
    printf '%s\n' "$GITLEAKS_BIN"
    return 0
  fi

  gitleaks_bin="$HOME/.local/bin/gitleaks"
  if [ -x "$gitleaks_bin" ]; then
    printf '%s\n' "$gitleaks_bin"
    return 0
  fi

  command -v gitleaks || return 1
}

require_gitleaks_bin() {
  local context="$1" gitleaks_bin found_version

  gitleaks_bin="$(find_gitleaks_bin || true)"
  if [ -z "$gitleaks_bin" ] || [ ! -x "$gitleaks_bin" ]; then
    printf 'gitleaks v%s is required by %s but is unavailable; run scripts/install-gitleaks.sh.\n' \
      "$GITLEAKS_VERSION" "$context" >&2
    return 1
  fi

  found_version="$(read_gitleaks_version "$gitleaks_bin" || true)"
  if [ "$found_version" != "$GITLEAKS_VERSION" ]; then
    printf 'gitleaks v%s is required by %s but %s reports %s; run scripts/install-gitleaks.sh.\n' \
      "$GITLEAKS_VERSION" "$context" "$gitleaks_bin" "${found_version:-no verifiable Go build metadata}" >&2
    return 1
  fi

  printf '%s\n' "$gitleaks_bin"
}
