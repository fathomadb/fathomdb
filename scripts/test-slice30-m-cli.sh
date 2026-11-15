#!/usr/bin/env bash
# Build separate provider-free source-test artifacts and execute real MCP boundaries.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel)"
cd "$REPO_ROOT"
if [ -L .venv ] || [ ! -x .venv/bin/python ]; then
  printf 'Slice30 M source tests require this checkout-owned .venv; run setup-python-owned.sh --provision.\n' >&2
  exit 1
fi
slice30_m_target="${S30_TASK:-${CARGO_TARGET_DIR:-$REPO_ROOT/target}/slice30-m}"
TMPDIR="${TMPDIR:-$REPO_ROOT/.cache/agent/tmp}"
mkdir -p "$TMPDIR"
export TMPDIR
cargo build --locked --release --jobs 2 -p fathomdb-mcp --no-default-features \
  --bin fathomdb-mcp --target-dir "$slice30_m_target/target-providerfree"
cargo build --locked --release --jobs 2 -p fathomdb-mcp --no-default-features \
  --features test-seams --example slice30_m_helper --target-dir "$slice30_m_target/target-helper"
FATHOMDB_SLICE30_M_CLI="$(realpath "$slice30_m_target/target-providerfree/release/fathomdb-mcp")"
FATHOMDB_SLICE30_M_HELPER="$(realpath "$slice30_m_target/target-helper/release/examples/slice30_m_helper")"
export FATHOMDB_SLICE30_M_CLI FATHOMDB_SLICE30_M_HELPER
exec .venv/bin/python dev/experiments/0.8.29-slice30-measurements/m_native_tests.py -v
