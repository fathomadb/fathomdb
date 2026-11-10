#!/usr/bin/env bash
# Build all language surfaces. Concise on success, structured on failure.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/agent-output.sh
. "$SCRIPT_DIR/lib/agent-output.sh"
cd_repo_root

# Python — always build a source-bound wheel in the checkout-owned environment.
if [ -f src/python/pyproject.toml ]; then
  run_capped build-python bash "$SCRIPT_DIR/setup-python-owned.sh" --build
else
  skip_notice build-python "no src/python/pyproject.toml"
fi

# Rust
run_capped build-rust cargo build --workspace --quiet

# TypeScript — only if node_modules already installed.
if [ -d src/ts/node_modules ]; then
  run_capped build-ts bash -c 'cd src/ts && npm run --silent build'
else
  skip_notice build-ts "src/ts/node_modules not installed"
fi
