#!/usr/bin/env bash
# Provision only the dependencies consumed by agent-verify's heavy test tier.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_toplevel="$(git rev-parse --show-toplevel)" || exit 1
cd "$repo_toplevel" || exit 1

bash "$SCRIPT_DIR/setup-python-owned.sh" --check-ownership

if [ ! -f src/python/pyproject.toml ]; then
  echo "bootstrap-heavy: missing src/python/pyproject.toml" >&2
  exit 1
fi
if [ ! -f src/ts/package.json ]; then
  echo "bootstrap-heavy: missing src/ts/package.json" >&2
  exit 1
fi

# Fail on missing pinned prerequisites before the expensive native install.
bash "$SCRIPT_DIR/install-actionlint.sh"
bash "$SCRIPT_DIR/install-shellcheck.sh"
echo "Installing locked root developer dependencies..."
npm ci

echo "Installing Python heavy-test and pinned preflight dependencies into .venv..."
bash "$SCRIPT_DIR/setup-python-owned.sh" --provision
.venv/bin/python -c 'import pytest, hypothesis'

echo "Installing locked TypeScript test dependencies..."
(cd src/ts && npm ci)
