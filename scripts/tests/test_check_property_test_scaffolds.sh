#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
CHECK="$ROOT/scripts/check-property-test-scaffolds.py"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

make_tree() {
  base="$1"
  mkdir -p "$base/src/rust/crates/fathomdb-engine/tests" \
    "$base/src/rust/crates/fathomdb-schema/tests" "$base/src/python/tests"
  printf 'fn written_record_identity_survives_reopen() {}\n' >"$base/src/rust/crates/fathomdb-engine/tests/property_template.rs"
  printf 'fn migration_suffix_is_contiguous_and_ends_at_head() {}\n' >"$base/src/rust/crates/fathomdb-schema/tests/property_template.rs"
  printf 'def test_written_record_identity_survives_reopen(): pass\n' >"$base/src/python/tests/test_property_template.py"
}

make_tree "$TMP/good"
python3 "$CHECK" --root "$TMP/good" >/dev/null
make_tree "$TMP/bad"
printf '\ndef test_placeholder_identity(x):\n    assert x == x\n' >>"$TMP/bad/src/python/tests/test_property_template.py"
if python3 "$CHECK" --root "$TMP/bad" >/dev/null 2>&1; then
  echo 'FAIL trivial property scaffold passed' >&2
  exit 1
fi
echo 'All property scaffold tests passed'
