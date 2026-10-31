#!/usr/bin/env bash
# AC42-11: build the Unit I page-cache tests (AC40-95 to AC40-97) from a crate
# outside the workspace, so no root [patch] applies and Cargo resolves the
# registry libsqlite3-sys exactly as a published-crate consumer does.
#
# usage: run.sh TREE RUSQLITE_REQ OUT_DIR
#   TREE          an exported FathomDB tree (git archive <sha> | tar -x)
#   RUSQLITE_REQ  the rusqlite requirement that tree uses ("0.40" or "0.41")
#   OUT_DIR       scratch directory for the consumer crate and its target
set -euo pipefail
tree="$(cd "$1" && pwd)"
rusqlite_req="$2"
out="$3"
if [ -n "${LIBSQLITE3_FLAGS:-}" ]; then
  echo "LIBSQLITE3_FLAGS is set; unset it so the registry build is measured" >&2
  exit 2
fi
mkdir -p "$out/src" "$out/tests"
cat >"$out/Cargo.toml" <<TOML
[package]
name = "slice42-registry-consumer"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
fathomdb-engine = { path = "$tree/src/rust/crates/fathomdb-engine", features = ["test-hooks"] }
fathomdb-schema = { path = "$tree/src/rust/crates/fathomdb-schema" }

[dev-dependencies]
rusqlite = { version = "$rusqlite_req", features = ["bundled", "fallible_uint", "hooks"] }
serde_json = "1"
tempfile = "3"
TOML
: >"$out/src/lib.rs"
cp "$tree/src/rust/crates/fathomdb-engine/tests/slice40_reader_page_cache.rs" "$out/tests/"
cat >"$out/tests/sqlite_version.rs" <<'RS'
#[test]
fn print_sqlite_version() {
    let mm = unsafe { rusqlite::ffi::sqlite3_compileoption_used(c"ENABLE_MEMORY_MANAGEMENT".as_ptr()) };
    println!(
        "SQLITE_VERSION {} {} ENABLE_MEMORY_MANAGEMENT={}",
        rusqlite::version_number(),
        rusqlite::version(),
        mm
    );
}
RS
cd "$out"
cargo generate-lockfile
cargo tree -i libsqlite3-sys
CARGO_TARGET_DIR="$out/target" cargo test --test sqlite_version -- --nocapture
CARGO_TARGET_DIR="$out/target" cargo test --test slice40_reader_page_cache -- --nocapture
