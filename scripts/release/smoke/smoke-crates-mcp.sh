#!/usr/bin/env bash
# scripts/release/smoke/smoke-crates-mcp.sh — AC-056 crates.io MCP server smoke.
#
#   $1 = version (e.g. 0.8.32)
#
# Installs fathomdb-mcp from crates.io (NOT from this workspace), runs
# --check-config (valid + SC-01 fail-fast conflict), and completes a real
# stdio MCP initialize + tools/list (exact 7 read-only tools) + list_stores
# session with clean stdin-close exit 0.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  printf 'usage: %s <version>\n' "$0" >&2
  exit 2
fi
VERSION="$1"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  printf 'smoke-crates-mcp: invalid version "%s" — expected semver MAJOR.MINOR.PATCH[-PRERELEASE]\n' \
    "$VERSION" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
WORK="$(mktemp -d)"
smoke_complete=0
cleanup() {
  local status=$? cleanup_status=0
  trap - EXIT
  rm -rf "$WORK" || cleanup_status=$?
  # Bash 3 can enter EXIT with status 0 after a fatal nounset expansion.
  if [ "$status" -eq 0 ]; then
    if [ "$smoke_complete" -ne 1 ]; then status=1; else status=$cleanup_status; fi
  fi
  exit "$status"
}
trap cleanup EXIT

cargo install fathomdb-mcp --version "=$VERSION" --root "$WORK" --locked
MCP_BIN="$WORK/bin/fathomdb-mcp"
if [ ! -x "$MCP_BIN" ]; then
  printf 'smoke-crates-mcp: expected binary %s not installed\n' "$MCP_BIN" >&2
  exit 1
fi

python3 "$REPO_ROOT/scripts/release/smoke/mcp-stdio-smoke.py" --binary "$MCP_BIN"

smoke_complete=1
printf 'smoke-crates-mcp: ok — fathomdb-mcp %s installed from crates.io and passed stdio smoke\n' \
  "$VERSION"
