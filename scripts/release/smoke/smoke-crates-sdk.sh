#!/usr/bin/env bash
# Install and run the published Rust SDK in an external consumer.
set -euo pipefail

if [ "$#" -ne 1 ] || ! [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  printf 'usage: %s <semver-version>\n' "$0" >&2
  exit 2
fi
version="$1"
work="$(mktemp -d)"
smoke_complete=0
cleanup() {
  local status=$? cleanup_status=0
  trap - EXIT
  rm -rf "$work" || cleanup_status=$?
  # Bash 3 can enter EXIT with status 0 after a fatal nounset expansion.
  if [ "$status" -eq 0 ]; then
    if [ "$smoke_complete" -ne 1 ]; then status=1; else status=$cleanup_status; fi
  fi
  exit "$status"
}
trap cleanup EXIT
mkdir -p "$work/src"
cat > "$work/Cargo.toml" <<EOF
[package]
name = "fathomdb-sdk-registry-smoke"
version = "0.0.0"
edition = "2021"

[dependencies]
fathomdb-sdk = "=$version"
EOF
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$script_dir/registry-sdk.rs" "$work/src/main.rs"
(
  cd "$work"
  cargo generate-lockfile
  cargo build --locked --quiet --target-dir "$work/target"
  consumer="$work/target/debug/fathomdb-sdk-registry-smoke"
  "$consumer" "$work/smoke.sqlite" write
  "$consumer" "$work/smoke.sqlite" reopen
)
smoke_complete=1
printf 'smoke-crates-sdk: ok — fathomdb-sdk %s resolved from crates.io; independent write and reopen processes exited cleanly\n' "$version"
