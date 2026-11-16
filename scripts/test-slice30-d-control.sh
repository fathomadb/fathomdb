#!/usr/bin/env bash
# Explicit D private-guardian/entry controls; harmless children, no scientific cells.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel)"
cd "$REPO_ROOT"
if [ -z "${S30_TASK:-}" ] || [ ! -d "$S30_TASK/tmp" ]; then
  printf 'D live controls require owned S30_TASK/tmp, private cgroup delegation, and32/250/16GiB start floors.\n' >&2
  exit 1
fi
if [ -L .venv ] || [ ! -x .venv/bin/python ]; then
  printf 'D live controls require this checkout-owned .venv.\n' >&2
  exit 1
fi
cargo build --locked --release --jobs 1 -p fathomdb-engine --no-default-features \
  --features slice30-observer,test-hooks --example slice30_d_storage_diagnostic
S30_D_DIAGNOSTIC_BINARY="${CARGO_TARGET_DIR:-$REPO_ROOT/target}/release/examples/slice30_d_storage_diagnostic" \
  .venv/bin/python dev/experiments/0.8.29-slice30-measurements/d_storage_diagnostic_tests.py -v
systemd-run --user --wait --pipe --quiet --service-type=exec \
  --unit="fathomdb-d-diagnostic-controls-$$" --setenv="S30_TASK=$S30_TASK" \
  --property=Delegate=yes --property=MemoryMax=4294967296 --property=MemorySwapMax=0 \
  --property=RuntimeMaxSec=60 --property=KillMode=control-group --working-directory="$REPO_ROOT" \
  "$REPO_ROOT/.venv/bin/python" -m unittest discover \
  -s dev/experiments/0.8.29-slice30-measurements -p 'd_diagnostic_*_tests.py' -v
.venv/bin/python dev/experiments/0.8.29-slice30-measurements/d_guard_tests.py -v
.venv/bin/python dev/experiments/0.8.29-slice30-measurements/d_cancel_tests.py -v
.venv/bin/python dev/experiments/0.8.29-slice30-measurements/d_entry_tests.py -v
exec .venv/bin/python dev/experiments/0.8.29-slice30-measurements/d_environment_tests.py -v
