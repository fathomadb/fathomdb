#!/usr/bin/env bash
# check-tegra-pool-registry-build.sh exit status (0.8.28 Slice 31, R-CODE-4
# note 2): without --resolve-only the script must run (a), (b) and (c), so on
# a host that is not an aarch64 Linux Tegra board it refuses before doing any
# work instead of exiting 0 after (a) alone. FATHOMDB_TEGRA_RELEASE_FILE
# points the host check at a missing file, so the arm is the same on any host.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="$ROOT/scripts/check-tegra-pool-registry-build.sh"

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

set +e
OUT="$(FATHOMDB_TEGRA_RELEASE_FILE=/nonexistent/nv_tegra_release bash "$SCRIPT" 2>&1)"
RC=$?
set -e
[ "$RC" -eq 2 ] || fail "full mode on a non-Tegra host: expected exit 2, got rc=$RC: $OUT"
grep -Fq -- '--resolve-only' <<<"$OUT" || fail "the refusal does not name --resolve-only: $OUT"
if grep -Fq 'PASS  (a)' <<<"$OUT"; then
  fail "the refusal ran (a) first: $OUT"
fi
pass "full mode on a non-Tegra host refuses with exit 2 before (a)"

set +e
OUT="$(bash "$SCRIPT" --bogus 2>&1)"
RC=$?
set -e
[ "$RC" -eq 2 ] || fail "unknown argument: expected exit 2, got rc=$RC: $OUT"
pass "an unknown argument is a usage error"
