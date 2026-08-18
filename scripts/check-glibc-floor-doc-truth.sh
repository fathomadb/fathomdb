#!/usr/bin/env bash
# scripts/check-glibc-floor-doc-truth.sh — AC80-9.
#
# Asserts docs/compatibility/index.md states the same glibc floor
# scripts/release/glibc-floor-contract.sh declares, so the published claim
# cannot drift from the artifact again. Fails closed if the doc makes no
# floor claim at all.
#
# Usage: check-glibc-floor-doc-truth.sh [--doc <path>]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOC="$REPO_ROOT/docs/compatibility/index.md"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --doc)
      [ "$#" -ge 2 ] || { printf 'usage: %s [--doc <path>]\n' "$(basename "$0")" >&2; exit 2; }
      DOC="$2"
      shift 2
      ;;
    *)
      printf 'usage: %s [--doc <path>]\n' "$(basename "$0")" >&2
      exit 2
      ;;
  esac
done

if [ ! -f "$DOC" ]; then
  printf 'check-glibc-floor-doc-truth: no such file: %s\n' "$DOC" >&2
  exit 2
fi

# shellcheck source=release/glibc-floor-contract.sh
. "$SCRIPT_DIR/release/glibc-floor-contract.sh"

CLAIMED="$(grep -m1 -oE 'Measured glibc floor: [0-9]+\.[0-9]+' "$DOC" | grep -oE '[0-9]+\.[0-9]+' || true)"

if [ -z "$CLAIMED" ]; then
  printf 'FAIL check-glibc-floor-doc-truth: %s makes no "Measured glibc floor: X.Y" claim\n' "$DOC" >&2
  exit 1
fi

if [ "$CLAIMED" != "$GLIBC_FLOOR" ]; then
  printf 'FAIL check-glibc-floor-doc-truth: %s claims glibc floor %s, contract declares %s\n' \
    "$DOC" "$CLAIMED" "$GLIBC_FLOOR" >&2
  exit 1
fi

exit 0
