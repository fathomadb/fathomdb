#!/usr/bin/env bash
# Compile and execute the ordinary tiny G CLI observation contract, without models.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel)"
cd "$REPO_ROOT"
cargo build --locked --release -p fathomdb-sdk --features slice30-observer --example slice30_measure
slice30_target="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
export FATHOMDB_SLICE30_G_BINARY="$(realpath "$slice30_target/release/examples/slice30_measure")"
exec .venv/bin/python dev/experiments/0.8.29-slice30-measurements/g_runtime_tests.py -v
