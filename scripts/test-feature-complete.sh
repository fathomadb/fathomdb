#!/usr/bin/env bash
# Feature-complete test gate: run every test that `cargo test --workspace`
# cannot. Each crate's test binaries are listed under the workspace features,
# under every feature set in scripts/test-feature-matrix.toml, and under the
# union of the crate's host-buildable features; each test that appears only
# under a matrix set or a derived extra set (a feature-gated target or an
# item-level #[cfg(feature = ...)] test) runs once, under the smallest set that
# has it. Extra sets that differ from the matrix's [[extra]] sets fail the gate;
# `--write-matrix` rewrites them and runs nothing.
# Runs use CUDA on the two RTX 3090s (CUDA_DEVICE_ORDER=PCI_BUS_ID,
# CUDA_VISIBLE_DEVICES=0,1; never the K620) and real, pinned assets: the
# embedder and reranker caches (`fathomdb doctor warm-cache`), nomic weights,
# and the ONNX Runtime library and exported bge-small graph. Each run is
# `cargo test ... -- --exact <tests> --nocapture --test-threads=1` with
# FATHOMDB_REQUIRE_LIVE=1, under which a test whose provisioned prerequisite is
# missing panics instead of skipping. Opt-in experiments are excluded by their
# scripts/test-skip-allowlist.toml entries. A skip marker not allowlisted as
# benign-message, an ignored test not allowlisted as ignored-by-design, a stale
# allowlist entry, a test that does not build, or any failing test fails the
# gate.
#
# Not part of agent-verify (runtime). scripts/check.sh runs it when
# FATHOMDB_FEATURE_COMPLETE=1; release qualification runs it directly.
# Logic: scripts/lib/feature_complete.py.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly script_dir
exec python3 "${script_dir}/lib/feature_complete.py" "$@"
