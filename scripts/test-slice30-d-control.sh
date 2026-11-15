#!/usr/bin/env bash
# Explicit D private-guardian/entry controls; harmless children, no scientific cells.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel)"
cd "$REPO_ROOT"
if [ -z "${S30_TASK:-}" ] || [ ! -d "$S30_TASK/tmp" ]; then
  printf 'D live controls require owned S30_TASK/tmp, private cgroup delegation, quiet load<=2, and32/250/16GiB start floors.\n' >&2
  exit 1
fi
if [ -L .venv ] || [ ! -x .venv/bin/python ]; then
  printf 'D live controls require this checkout-owned .venv.\n' >&2
  exit 1
fi
.venv/bin/python dev/experiments/0.8.29-slice30-measurements/d_guard_tests.py -v
exec .venv/bin/python dev/experiments/0.8.29-slice30-measurements/d_entry_tests.py -v
