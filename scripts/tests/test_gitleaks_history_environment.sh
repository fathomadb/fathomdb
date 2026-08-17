#!/usr/bin/env bash
# The history guard, rather than the caller, owns Gitleaks configuration.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
HISTORY_GUARD="$REPO_ROOT/scripts/security/gitleaks-history.sh"
GITLEAKS_BIN="$(command -v gitleaks)"

failures=0

expect_nonzero() {
  local rc="$1" description="$2"
  if [ "$rc" -eq 0 ]; then
    printf 'FAIL  %s (expected non-zero exit)\n' "$description" >&2
    failures=$((failures + 1))
  else
    printf 'PASS  %s\n' "$description"
  fi
}

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
fixture="$tmp_root/repo"
mkdir -p "$fixture"
git -C "$fixture" init -q
git -C "$fixture" config user.email gitleaks-history-environment@example.invalid
git -C "$fixture" config user.name 'Gitleaks History Environment Test'
printf '%s\n' \
  '[[rules]]' \
  'id = "fixture-environment-history-rule"' \
  "regex = '''marker_[A-Z0-9]{24}'''" >"$fixture/.gitleaks.toml"
token="$(printf '%s%s%s%s%s%s%s' \
  'marker_' 'ABCD' 'EF12' '3456' '7890' 'ABCD' 'EF12')"
printf 'credential=%s\n' "$token" >"$fixture/history.txt"
git -C "$fixture" add .gitleaks.toml history.txt
git -C "$fixture" commit -qm 'fixture environment history'
printf '%s\n' '[allowlist]' "paths = ['.*']" >"$tmp_root/suppress-history.toml"

set +e
path_config_out="$(env -u GITLEAKS_CONFIG_TOML \
  GITLEAKS_CONFIG="$tmp_root/suppress-history.toml" \
  GITLEAKS_BIN="$GITLEAKS_BIN" "$HISTORY_GUARD" "$fixture" 2>&1)"
path_config_rc=$?
toml_config_out="$(env -u GITLEAKS_CONFIG \
  GITLEAKS_CONFIG_TOML=$'[allowlist]\npaths = [\'.*\']' \
  GITLEAKS_BIN="$GITLEAKS_BIN" "$HISTORY_GUARD" "$fixture" 2>&1)"
toml_config_rc=$?
set -e

expect_nonzero "$path_config_rc" "history guard ignores inherited GITLEAKS_CONFIG"
expect_nonzero "$toml_config_rc" "history guard ignores inherited GITLEAKS_CONFIG_TOML"
if [[ "$path_config_out" == *"$token"* || "$toml_config_out" == *"$token"* ]]; then
  printf '%s\n' 'FAIL  history guard redacts inherited-config fixture credentials from output' >&2
  failures=$((failures + 1))
else
  printf '%s\n' 'PASS  history guard redacts inherited-config fixture credentials from output'
fi

exit "$failures"
