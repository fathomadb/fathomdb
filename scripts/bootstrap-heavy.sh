#!/usr/bin/env bash
# Provision only the dependencies consumed by agent-verify's heavy test tier.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_toplevel="$(git rev-parse --show-toplevel)" || exit 1
cd "$repo_toplevel" || exit 1

# shellcheck source=lib/agent-python-env.sh
. "$SCRIPT_DIR/lib/agent-python-env.sh"

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
create_venv_with_selected_python .venv
.venv/bin/python -m pip install -e 'src/python[dev]'
.venv/bin/python -c 'import pytest, hypothesis'

echo "Installing locked TypeScript test dependencies..."
(cd src/ts && npm ci)
