#!/usr/bin/env bash
# Compile-check scripts/release/smoke/registry-sdk.rs against the local workspace
# fathomdb-sdk crate so Rust SDK struct changes never break production
# post-publish-smoke (crates-sdk) after tagging.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SMOKE_RS="$REPO_ROOT/scripts/release/smoke/registry-sdk.rs"

if [ ! -f "$SMOKE_RS" ]; then
  printf 'check-release-smoke-sdk-compile: missing %s\n' "$SMOKE_RS" >&2
  exit 1
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-smoke-sdk-check-XXXXXX")"
cleanup() { rm -rf "$work"; }
trap cleanup EXIT

mkdir -p "$work/src"
cp "$SMOKE_RS" "$work/src/main.rs"
cp "$REPO_ROOT/Cargo.lock" "$work/Cargo.lock"
cat > "$work/Cargo.toml" <<EOF
[package]
name = "fathomdb-sdk-registry-smoke-check"
version = "0.0.0"
edition = "2021"

[dependencies]
fathomdb-sdk = { path = "$REPO_ROOT/src/rust/crates/fathomdb-sdk" }
EOF

cargo check --offline --quiet \
  --manifest-path "$work/Cargo.toml" \
  --target-dir "$REPO_ROOT/target"
