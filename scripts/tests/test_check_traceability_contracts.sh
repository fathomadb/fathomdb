#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
CHECK="$ROOT/scripts/check-traceability-contracts.py"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fixture() {
  dir="$1"; reference="$2"
  mkdir -p "$dir/dev"
  printf 'NEED-001: **Trustworthy storage.**\n' >"$dir/dev/needs.md"
  printf '| NEED | Summary |\n| --- | --- |\n| %s | row |\n' "$reference" >"$dir/dev/traceability.md"
}

fixture "$TMP/good" NEED-001
python3 "$CHECK" --root "$TMP/good" >/dev/null
fixture "$TMP/missing" NEED-026
if python3 "$CHECK" --root "$TMP/missing" >/dev/null 2>&1; then echo 'FAIL missing need passed'; exit 1; fi
fixture "$TMP/malformed" NEED-XYZ
if python3 "$CHECK" --root "$TMP/malformed" >/dev/null 2>&1; then echo 'FAIL malformed need passed'; exit 1; fi
echo 'All traceability contract tests passed'
