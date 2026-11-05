#!/usr/bin/env bash
# Shared exact-version Actionlint provisioning for developer and heavy CI setup.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/actionlint-version.sh
. "$SCRIPT_DIR/lib/actionlint-version.sh"

readonly ACTIONLINT_VERSION="1.7.12"
actionlint_bin="$(find_actionlint_bin || true)"
actionlint_version=""
if [ -n "$actionlint_bin" ]; then
  actionlint_version="$(read_actionlint_version "$actionlint_bin")"
fi

if [ "$actionlint_version" != "$ACTIONLINT_VERSION" ]; then
  if ! command -v go >/dev/null 2>&1; then
    echo "actionlint $ACTIONLINT_VERSION is required but go is unavailable; install it manually" >&2
    echo "  see https://github.com/rhysd/actionlint/releases (pin v$ACTIONLINT_VERSION)" >&2
    exit 1
  fi
  echo "Installing actionlint v$ACTIONLINT_VERSION via go install..."
  GO111MODULE=on go install "github.com/rhysd/actionlint/cmd/actionlint@v$ACTIONLINT_VERSION"
  actionlint_bin="$(go env GOPATH)/bin/actionlint"
  installed_actionlint_version=""
  if [ -x "$actionlint_bin" ]; then
    installed_actionlint_version="$(read_actionlint_version "$actionlint_bin")"
  fi
  if [ "$installed_actionlint_version" != "$ACTIONLINT_VERSION" ]; then
    echo "actionlint v$ACTIONLINT_VERSION installation did not produce the required binary" >&2
    exit 1
  fi
  echo "actionlint v$ACTIONLINT_VERSION is installed at $actionlint_bin"
fi

# GitHub Actions applies GITHUB_PATH only to later steps. Persist the resolved
# actionlint directory so `agent-verify` can invoke the exact bootstrap-installed
# binary. (install-shellcheck.sh persists shellcheck's own directory.)
if [ -n "${GITHUB_PATH:-}" ]; then
  actionlint_dir="$(dirname "$actionlint_bin")"
  printf '%s\n' "$actionlint_dir" >>"$GITHUB_PATH"
fi
