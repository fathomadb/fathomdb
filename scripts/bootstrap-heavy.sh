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

echo "Installing Python heavy-test dependencies into .venv..."
create_venv_with_selected_python .venv
.venv/bin/python -m pip install -e 'src/python[test]'
.venv/bin/python -c 'import pytest, hypothesis'

echo "Installing locked TypeScript test dependencies..."
(cd src/ts && npm ci)
