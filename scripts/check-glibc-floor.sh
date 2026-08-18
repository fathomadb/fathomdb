#!/usr/bin/env bash
# scripts/check-glibc-floor.sh — AC80-1/AC80-2/R80-2.
#
# Asserts that native artifacts (.node bindings, _fathomdb.abi3.so wheels)
# require no GLIBC symbol version above a declared floor. Prefers
# `objdump -T`; falls back to `readelf -V` when objdump is unavailable.
# Fails closed — the same discipline scripts/check-license-consistency.sh's
# tomllib gate uses — when neither tool is present: a silent pass here would
# report a check it never actually ran.
#
# Usage: check-glibc-floor.sh --floor X.Y <file> [file...]
set -euo pipefail

usage() {
  printf 'usage: %s --floor X.Y <file> [file...]\n' "$(basename "$0")" >&2
}

FLOOR=""
FILES=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --floor)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      FLOOR="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      FILES+=("$1")
      shift
      ;;
  esac
done

if [ -z "$FLOOR" ] || [ "${#FILES[@]}" -eq 0 ]; then
  usage
  exit 2
fi

if ! [[ "$FLOOR" =~ ^[0-9]+\.[0-9]+$ ]]; then
  printf 'check-glibc-floor: --floor must be MAJOR.MINOR (got %s)\n' "$FLOOR" >&2
  exit 2
fi

TOOL=""
if command -v objdump >/dev/null 2>&1; then
  TOOL="objdump"
elif command -v readelf >/dev/null 2>&1; then
  TOOL="readelf"
else
  printf 'check-glibc-floor: neither objdump nor readelf is available on PATH — refusing to report a pass it did not verify\n' >&2
  exit 2
fi

# Compares two MAJOR.MINOR[.PATCH] version strings. Echoes the greater one.
max_version() {
  printf '%s\n%s\n' "$1" "$2" | sort -t. -k1,1n -k2,2n -k3,3n | tail -n1
}

FAILED=0
for f in "${FILES[@]}"; do
  if [ ! -e "$f" ]; then
    printf 'check-glibc-floor: %s: no such file\n' "$f" >&2
    exit 2
  fi

  if [ "$TOOL" = "objdump" ]; then
    RAW="$(objdump -T "$f" 2>/dev/null || true)"
  else
    RAW="$(readelf -V "$f" 2>/dev/null || true)"
  fi

  VERSIONS="$(printf '%s\n' "$RAW" | grep -oE 'GLIBC_[0-9]+\.[0-9]+(\.[0-9]+)?' | sed 's/^GLIBC_//' || true)"
  MAX=""
  while IFS= read -r ver; do
    [ -z "$ver" ] && continue
    if [ -z "$MAX" ]; then
      MAX="$ver"
    else
      MAX="$(max_version "$MAX" "$ver")"
    fi
  done <<< "$VERSIONS"

  if [ -z "$MAX" ]; then
    continue
  fi

  TOP="$(max_version "$MAX" "$FLOOR")"
  if [ "$TOP" != "$FLOOR" ]; then
    printf 'FAIL check-glibc-floor: %s requires GLIBC_%s, above the declared floor GLIBC_%s\n' \
      "$f" "$MAX" "$FLOOR" >&2
    FAILED=1
  fi
done

if [ "$FAILED" -ne 0 ]; then
  exit 1
fi

exit 0
