#!/usr/bin/env bash
# Feature-complete test gate (0.8.27 RH-14): run every workspace test target
# that `cargo test --workspace` cannot, once per required feature set in
# scripts/test-feature-matrix.toml, with CUDA on the two RTX 3090s
# (CUDA_DEVICE_ORDER=PCI_BUS_ID, CUDA_VISIBLE_DEVICES=0,1; never the K620) and
# real model weights (`fathomdb doctor warm-cache`, then pinned
# nomic-embed-text-v1.5 weights for `nomic_smoke`). Each run is
# `cargo test ... -- --nocapture --test-threads=1`; a skip marker or ignored
# test outside scripts/test-skip-allowlist.toml, a stale allowlist entry, or
# any failing test fails the gate.
#
# Not part of agent-verify (runtime). scripts/check.sh runs it when
# FATHOMDB_FEATURE_COMPLETE=1; Slice 70, Slice 150, and release qualification
# run it directly. Logic: scripts/lib/feature_complete.py.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly script_dir
exec python3 "${script_dir}/lib/feature_complete.py" "$@"
