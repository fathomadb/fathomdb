#!/usr/bin/env bash
# scripts/release/smoke/smoke-crates-cli.sh — AC-056 crates.io smoke.
#
#   $1 = version (e.g. 0.6.0)
#
# Installs fathomdb-cli from crates.io (NOT from this workspace), creates a
# fresh fixture DB in a tempdir, verifies binary identity, runs both integrity
# commands, and proves the immutable data-plane command leaves the complete
# product file set unchanged. Engine::open creates the DB file lazily, so the
# smoke does not need an init verb.
#
# Per `feedback_release_verification`: green CI + published artifact is NOT
# done. This script is the gate that proves the published crate actually
# installs and runs cleanly on a fresh ubuntu — distinct from the CI build,
# which only proves the workspace tree compiles.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  printf 'usage: %s <version>\n' "$0" >&2
  exit 2
fi
VERSION="$1"
if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'; then
  printf 'smoke-crates-cli: invalid version "%s" — expected semver MAJOR.MINOR.PATCH[-PRERELEASE]\n' \
    "$VERSION" >&2
  exit 2
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# `cargo install` honors --root for the install prefix; binary lands at
# $WORK/bin/fathomdb.
cargo install fathomdb-cli --version "=$VERSION" --root "$WORK" --locked
VERSION_OUT="$("$WORK/bin/fathomdb" --version)"
if [ "$VERSION_OUT" != "fathomdb $VERSION" ]; then
  printf 'smoke-crates-cli: version mismatch — expected "fathomdb %s", got "%s"\n' \
    "$VERSION" "$VERSION_OUT" >&2
  exit 1
fi

DB="$WORK/smoke.fdb"
OUT="$("$WORK/bin/fathomdb" doctor check-integrity --json "$DB")"
printf '%s\n' "$OUT" | jq -e . >/dev/null

snapshot_product_files() {
  for suffix in '' '.lock' '-wal' '-shm' '-journal'; do
    candidate="${DB}${suffix}"
    if [ -f "$candidate" ]; then
      printf '%s present ' "$suffix"
      sha256sum "$candidate" | cut -d' ' -f1
    else
      printf '%s absent\n' "$suffix"
    fi
  done
}

BEFORE="$(snapshot_product_files)"
SLICE55_OUT="$("$WORK/bin/fathomdb" doctor data-plane-integrity --json "$DB")"
printf '%s\n' "$SLICE55_OUT" | jq -e \
  '.schemaVersion == "fathomdb.doctor.data-plane-integrity.v1" and .status == "clean"' >/dev/null
AFTER="$(snapshot_product_files)"
if [ "$BEFORE" != "$AFTER" ]; then
  printf 'smoke-crates-cli: data-plane inspection changed product files\n' >&2
  exit 1
fi

printf 'smoke-crates-cli: ok — fathomdb-cli %s identified; legacy and immutable integrity returned valid JSON\n' \
  "$VERSION"
