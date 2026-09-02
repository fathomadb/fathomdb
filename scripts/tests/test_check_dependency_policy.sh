#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
CHECK="$ROOT/scripts/check-dependency-policy.py"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

make_fixture() {
  dir="$1"; httpmock="$2"; include_async="$3"; event="$4"
  mkdir -p "$dir/src/rust/crates/fathomdb-embedder"
  printf '[dev-dependencies]\nhttpmock = "%s"\n' "$httpmock" >"$dir/src/rust/crates/fathomdb-embedder/Cargo.toml"
  {
    printf 'version = 4\n'
    for pair in "httpmock 0.8.3" "anyhow 1.0.103" "crossbeam-epoch 0.9.20" "memmap2 0.9.11" "event-listener $event"; do
      set -- $pair
      printf '[[package]]\nname = "%s"\nversion = "%s"\n' "$1" "$2"
    done
    if [ "$include_async" = yes ]; then
      printf '[[package]]\nname = "async-std"\nversion = "1.13.2"\n'
    fi
  } >"$dir/Cargo.lock"
}

make_fixture "$TMP/good" '=0.8.3' no 5.4.2
python3 "$CHECK" --root "$TMP/good" >/dev/null || { echo 'FAIL valid dependency policy'; exit 1; }

make_fixture "$TMP/bad-pin" '0.8' no 5.4.2
if python3 "$CHECK" --root "$TMP/bad-pin" >/dev/null 2>&1; then echo 'FAIL loose httpmock pin passed'; exit 1; fi

make_fixture "$TMP/bad-async" '=0.8.3' yes 5.4.2
if python3 "$CHECK" --root "$TMP/bad-async" >/dev/null 2>&1; then echo 'FAIL async-std passed'; exit 1; fi

make_fixture "$TMP/bad-event" '=0.8.3' no 5.4.1
if python3 "$CHECK" --root "$TMP/bad-event" >/dev/null 2>&1; then echo 'FAIL stale event-listener passed'; exit 1; fi

echo 'All dependency policy tests passed'
