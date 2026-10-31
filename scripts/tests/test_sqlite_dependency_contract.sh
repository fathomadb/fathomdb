#!/usr/bin/env bash
# The SQLite stack is coupled: every direct rusqlite consumer uses 0.41
# (0.8.28 Slice 42: crates.io libsqlite3-sys 0.39, SQLite 3.54.1) and both
# vector paths retain the sqlite-vec 0.1.9 TC-76 tripwire.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
for manifest in \
  src/rust/crates/fathomdb-cli/Cargo.toml \
  src/rust/crates/fathomdb-engine/Cargo.toml \
  src/rust/crates/fathomdb-schema/Cargo.toml; do
  if ! grep -Fq 'rusqlite = { version = "0.41", features = ["bundled", "fallible_uint"] }' "$root/$manifest"; then
    printf 'FAIL  %s must use bundled rusqlite 0.41 with fallible unsigned SQLite bindings\n' "$manifest" >&2
    exit 1
  fi
done
for manifest in \
  src/rust/crates/fathomdb-engine/Cargo.toml \
  src/rust/crates/fathomdb-schema/Cargo.toml; do
  if ! grep -Fq 'sqlite-vec = "=0.1.9"' "$root/$manifest"; then
    printf 'FAIL  %s must pin sqlite-vec 0.1.9\n' "$manifest" >&2
    exit 1
  fi
done

tc76="$root/src/rust/crates/fathomdb-engine/tests/tc76_vec0_long_metadata_delete.rs"
if ! grep -Fq 'fn bare_vec0_delete_pins_the_upstream_length_boundary' "$tc76" \
  || ! grep -Fq 'sqlite-vec 0.1.9 must delete a' "$tc76" \
  || ! grep -Fq 'assert_eq!(residue, 0' "$tc76"; then
  printf 'FAIL  TC-76 must prove sqlite-vec 0.1.9 deletes long vec0 metadata without residue\n' >&2
  exit 1
fi
printf 'PASS  sqlite dependency contract (rusqlite 0.41) and long-metadata TC-76 delete tripwire\n'
