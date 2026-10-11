#!/usr/bin/env bash
# Install and run the published Rust SDK in an external consumer.
set -euo pipefail

if [ "$#" -ne 1 ] || ! [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  printf 'usage: %s <semver-version>\n' "$0" >&2
  exit 2
fi
version="$1"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/src"
cat > "$work/Cargo.toml" <<EOF
[package]
name = "fathomdb-sdk-registry-smoke"
version = "0.0.0"
edition = "2021"

[dependencies]
fathomdb-sdk = "=$version"
EOF
cat > "$work/src/main.rs" <<'EOF'
use fathomdb_sdk::{Engine, OpenOptions};

fn main() -> fathomdb_sdk::Result<()> {
    let path = std::env::args().nth(1).expect("database path");
    let engine = Engine::open(path, OpenOptions::default())?;
    engine.close()?;
    Ok(())
}
EOF
(
  cd "$work"
  cargo generate-lockfile
  cargo run --locked --quiet -- "$work/smoke.sqlite"
)
printf 'smoke-crates-sdk: ok — fathomdb-sdk %s resolved from crates.io; external consumer opened and closed\n' "$version"
