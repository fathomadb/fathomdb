#!/usr/bin/env bash
# Enforce the reviewed fathomdb-engine module ownership graph.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOL_DIR="$REPO_ROOT/dev/tools/module-boundary-gate"
MANIFEST="$TOOL_DIR/Cargo.toml"
LOCKFILE="$TOOL_DIR/Cargo.lock"
BINARY="$TOOL_DIR/target/debug/fathomdb-module-boundary-gate"
CARGO_BIN="${FATHOMDB_MODULE_BOUNDARY_CARGO:-cargo}"
newer_source="$(find "$TOOL_DIR/src" -type f -name '*.rs' -newer "$BINARY" -print -quit 2>/dev/null || true)"

if ! grep -Fqx 'license = "MIT"' "$MANIFEST" ||
  ! grep -Fqx 'proc-macro2 = { version = "1.0.106", features = ["span-locations"] }' "$MANIFEST" ||
  ! grep -Fqx 'syn = { version = "2.0.117", default-features = false, features = ["full", "parsing", "visit"] }' "$MANIFEST"; then
  printf 'FAIL module-boundary: standalone dependency/license contract drifted in %s\n' "$MANIFEST" >&2
  exit 1
fi

stale=0
if [ ! -x "$BINARY" ]; then
  stale=1
elif [ "$MANIFEST" -nt "$BINARY" ] || [ "$LOCKFILE" -nt "$BINARY" ]; then
  stale=1
elif [ -n "$newer_source" ]; then
  stale=1
fi

if [ "$stale" -eq 1 ]; then
  "$CARGO_BIN" build --quiet --locked --manifest-path "$MANIFEST"
fi

exec "$BINARY" --root "$REPO_ROOT"
