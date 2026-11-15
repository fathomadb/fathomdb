#!/usr/bin/env bash
# Explicit live guardian control; separate from tiny portable product CLI tests.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel)"
cd "$REPO_ROOT"
if [ -z "${S30_TASK:-}" ] || [ ! -d "$S30_TASK/tmp" ]; then
  printf 'M live guardian control requires an owned S30_TASK with tmp/, private cgroup delegation, and32/250/16GiB start floors. Run in the declared owned executor.\n' >&2
  exit 1
fi
if [ -L .venv ] || [ ! -x .venv/bin/python ]; then
  printf 'M live guardian control requires this checkout-owned .venv.\n' >&2
  exit 1
fi
FATHOMDB_SLICE30_M_CLI="$S30_TASK/target-providerfree/release/fathomdb-mcp"
FATHOMDB_SLICE30_M_HELPER="$S30_TASK/target-helper/release/examples/slice30_m_helper"
if [ ! -x "$FATHOMDB_SLICE30_M_CLI" ] || [ ! -x "$FATHOMDB_SLICE30_M_HELPER" ]; then
  printf 'M live dispatch control requires the owned artifacts from scripts/test-slice30-m-cli.sh first.\n' >&2
  exit 1
fi
TMPDIR="${TMPDIR:-$S30_TASK/tmp}"
export FATHOMDB_SLICE30_M_CLI FATHOMDB_SLICE30_M_HELPER TMPDIR
.venv/bin/python dev/experiments/0.8.29-slice30-measurements/m_guard_tests.py -v
exec .venv/bin/python dev/experiments/0.8.29-slice30-measurements/m_dispatch_tests.py -v
