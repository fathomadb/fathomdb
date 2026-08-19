#!/usr/bin/env bash
# scripts/check-glibc-floor-doc-truth.sh — AC80-9, AC80-26.
#
# Asserts docs/compatibility/index.md states the same glibc floor
# scripts/release/glibc-floor-contract.sh declares FOR EVERY DECLARED ARTIFACT
# FAMILY, so the published claim cannot drift from the artifact again. Fails
# closed if the doc makes no claim for a family.
#
# 0.8.23 Slice 80.6 (D-80.6-5): this gate used to read the doc with
# `grep -m1 -oE 'Measured glibc floor: [0-9]+\.[0-9]+'` — FIRST MATCH ONLY —
# and compare it against a single $GLIBC_FLOOR. With two families stated in the
# doc, that shape checks the first claim and lets a wrong second claim pass
# silently, which is precisely the drift AC80-9 exists to prevent. The claims
# are therefore per-family markered and each is checked, and EVERY occurrence
# of a family's marker must agree — so a contradictory repeat cannot hide
# behind the first correct one either.
#
# The doc must carry, once per family, a claim of the form:
#   Measured glibc floor (<family>): X.Y
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

if [ -z "${GLIBC_FLOOR_FAMILIES:-}" ]; then
  printf 'FAIL check-glibc-floor-doc-truth: the contract declares no artifact families\n' >&2
  exit 1
fi

STATUS=0

for family in $GLIBC_FLOOR_FAMILIES; do
  if ! declared="$(glibc_floor_for_family "$family")"; then
    printf 'FAIL check-glibc-floor-doc-truth: contract declares family %s with no floor\n' \
      "$family" >&2
    STATUS=1
    continue
  fi

  claims="$(grep -oE "Measured glibc floor \($family\): [0-9]+\.[0-9]+" "$DOC" \
    | grep -oE '[0-9]+\.[0-9]+' || true)"

  if [ -z "$claims" ]; then
    printf 'FAIL check-glibc-floor-doc-truth: %s makes no "Measured glibc floor (%s): X.Y" claim; the contract declares %s\n' \
      "$DOC" "$family" "$declared" >&2
    STATUS=1
    continue
  fi

  # Every occurrence must agree with the contract, not merely the first one.
  for claimed in $claims; do
    if [ "$claimed" != "$declared" ]; then
      printf 'FAIL check-glibc-floor-doc-truth: %s claims glibc floor %s for family %s, contract declares %s\n' \
        "$DOC" "$claimed" "$family" "$declared" >&2
      STATUS=1
    fi
  done
done

exit "$STATUS"
