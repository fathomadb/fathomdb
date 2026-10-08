#!/usr/bin/env bash
# The CUDA memory-pool study's scaffolding stays out of the product (0.8.28
# Slice 30, R30-10 / AC30-10): its environment variables and its experiment
# report-line tag must not appear in any tracked file outside `dev/`, where the
# study and its history live.
#
# The names are assembled at run time so this file does not match itself.
# A self-test first proves the scan fails on an injected occurrence and
# ignores `dev/`.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PREFIX='FATHOMDB_POOL_'
PATTERN="${PREFIX}VARIANT|${PREFIX}TRIM|${PREFIX}STATS_EVERY_S|${PREFIX}COEXIST_CHECK|${PREFIX}MAPS_DIR|fdb-pool""-exp"

# scan ROOT: print every tracked occurrence outside dev/; exit 1 if any.
scan() {
  local root="$1"
  if git -C "$root" grep -n -I -E "$PATTERN" -- . ':(exclude)dev/'; then
    return 1
  fi
  return 0
}

SELFTEST="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-pool-scaffolding.XXXXXX")"
trap 'rm -rf "$SELFTEST"' EXIT
git -C "$SELFTEST" init -q
mkdir -p "$SELFTEST/dev/plans" "$SELFTEST/src"
printf 'history: %s=b\n' "${PREFIX}VARIANT" >"$SELFTEST/dev/plans/study.md"
printf 'fn main() {}\n' >"$SELFTEST/src/lib.rs"
git -C "$SELFTEST" add -A
if ! scan "$SELFTEST" >/dev/null; then
  printf 'FAIL  the scan flagged an occurrence under dev/\n' >&2
  exit 1
fi
printf 'PASS  self-test: occurrences under dev/ are allowed\n'
printf 'std::env::var("%s")\n' "${PREFIX}TRIM" >>"$SELFTEST/src/lib.rs"
git -C "$SELFTEST" add -A
if scan "$SELFTEST" >/dev/null; then
  printf 'FAIL  the scan missed an injected occurrence outside dev/\n' >&2
  exit 1
fi
printf 'PASS  self-test: an injected occurrence outside dev/ fails\n'

if ! scan "$REPO_ROOT"; then
  printf 'FAIL  pool-study scaffolding remains outside dev/ (R30-10)\n' >&2
  exit 1
fi
printf 'PASS  no pool-study scaffolding outside dev/\n'
