#!/usr/bin/env bash
# RED-first coverage for the glibc-floor gate (AC80-1, AC80-2).
#
# check-glibc-floor.sh must fail closed when neither objdump nor readelf is
# available (the same discipline the tomllib gates use — see
# scripts/check-license-consistency.sh), and must correctly flag a fixture
# native object that requires a GLIBC version above the declared floor.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-glibc-floor.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

BASH_BIN="$(command -v bash)"

# A fake native object; content is irrelevant because the fake objdump/readelf
# stubs below never actually inspect it. What matters is the checker parses
# stub output correctly.
FIXTURE="$WORK/fixture.node"
: > "$FIXTURE"

make_bin_dir() {
  local name="$1"
  local dir="$WORK/bin-$name"
  mkdir -p "$dir"
  printf '%s' "$dir"
}

# A PATH dir carrying only the coreutils check-glibc-floor.sh itself needs
# (grep/sed/sort/cat/tail), deliberately excluding objdump and readelf, so
# tool-absence can be simulated without depending on what happens to be
# installed elsewhere on PATH.
CORE_DIR="$WORK/core-utils"
mkdir -p "$CORE_DIR"
for tool in grep sed sort cat tail basename tr; do
  t="$(command -v "$tool" || true)"
  [ -n "$t" ] && ln -s "$t" "$CORE_DIR/$tool"
done
run_checker() {
  local dir="$1"
  shift
  PATH="$dir:$CORE_DIR" "$BASH_BIN" "$CHECKER" "$@"
}

# --- Case 1: objdump reports a symbol above the floor -> gate must FAIL ---
BIN1="$(make_bin_dir over-floor)"
cat > "$BIN1/objdump" <<'SH'
#!/bin/sh
cat <<'EOF'
0000000000012340 g    DF .text  0000000000000050  GLIBC_2.34  pthread_kill
0000000000012350 g    DF .text  0000000000000050  GLIBC_2.39  __isoc23_strtol
0000000000012360 g    DF .text  0000000000000050  GLIBC_2.2.5  printf
EOF
SH
chmod +x "$BIN1/objdump"
set +e
OUT1="$(run_checker "$BIN1" --floor 2.28 "$FIXTURE" 2>&1)"
RC1=$?
set -e
if [ "$RC1" -eq 1 ] && [[ "$OUT1" == *"GLIBC_2.39"* ]]; then
  pass "over-floor fixture fails with the offending symbol version named"
else
  fail "over-floor fixture: expected exit 1 naming GLIBC_2.39, got rc=$RC1 out=$OUT1"
fi

# --- Case 2: objdump reports only symbols at/under the floor -> gate PASSes ---
BIN2="$(make_bin_dir under-floor)"
cat > "$BIN2/objdump" <<'SH'
#!/bin/sh
cat <<'EOF'
0000000000012340 g    DF .text  0000000000000050  GLIBC_2.28  pthread_kill
0000000000012360 g    DF .text  0000000000000050  GLIBC_2.2.5  printf
EOF
SH
chmod +x "$BIN2/objdump"
set +e
OUT2="$(run_checker "$BIN2" --floor 2.28 "$FIXTURE" 2>&1)"
RC2=$?
set -e
if [ "$RC2" -eq 0 ]; then
  pass "under-floor fixture passes"
else
  fail "under-floor fixture: expected exit 0, got rc=$RC2 out=$OUT2"
fi

# --- Case 3: neither objdump nor readelf on PATH -> fail closed, distinct diagnostic ---
BIN3="$(make_bin_dir no-tools)"
set +e
OUT3="$(run_checker "$BIN3" --floor 2.28 "$FIXTURE" 2>&1)"
RC3=$?
set -e
if [ "$RC3" -eq 2 ] && [[ "${OUT3,,}" == *"neither objdump nor readelf"* ]]; then
  pass "missing-tool case fails closed with distinct diagnostic"
else
  fail "missing-tool case: expected exit 2 naming missing tools, got rc=$RC3 out=$OUT3"
fi

# --- Case 4: objdump absent, readelf fallback used and correctly flags a violation ---
BIN4="$(make_bin_dir readelf-fallback)"
cat > "$BIN4/readelf" <<'SH'
#!/bin/sh
cat <<'EOF'
Version needs section '.gnu.version_r' contains 2 entries:
 Addr: 0x0000000000000000  Offset: 0x000000  Link: 0
  000000: Version: 1  File: libc.so.6  Cnt: 2
  0x0010:   Name: GLIBC_2.39  Flags: none  Version: 3
  0x0020:   Name: GLIBC_2.2.5  Flags: none  Version: 2
EOF
SH
chmod +x "$BIN4/readelf"
set +e
OUT4="$(run_checker "$BIN4" --floor 2.28 "$FIXTURE" 2>&1)"
RC4=$?
set -e
if [ "$RC4" -eq 1 ] && [[ "$OUT4" == *"GLIBC_2.39"* ]]; then
  pass "readelf fallback used when objdump is absent, correctly flags violation"
else
  fail "readelf fallback: expected exit 1 naming GLIBC_2.39, got rc=$RC4 out=$OUT4"
fi

# --- Case 5: usage error when no files given ---
set +e
OUT5="$("$BASH_BIN" "$CHECKER" --floor 2.28 2>&1)"
RC5=$?
set -e
if [ "$RC5" -eq 2 ]; then
  pass "no files given is a usage error"
else
  fail "no files given: expected exit 2, got rc=$RC5 out=$OUT5"
fi

# --- 0.8.23 Slice 80.6 (D-80.6-5, AC80-26): the floor is declared PER
# ARTIFACT FAMILY. check-glibc-floor.sh itself is unchanged — it already takes
# an arbitrary --floor X.Y — so what needs covering is the contract that
# chooses the floor, and the guarantee that every pre-80.6 call site keeps the
# floor it had. CONTRACT_UNDER_TEST lets these arms be pointed at an older
# contract to prove they are RED against it.
CONTRACT="${CONTRACT_UNDER_TEST:-$REPO_ROOT/scripts/release/glibc-floor-contract.sh}"

read_contract() {
  # shellcheck disable=SC1090
  ( set -euo pipefail
    . "$CONTRACT"
    printf '%s|%s|%s|%s\n' \
      "${GLIBC_FLOOR:-<unset>}" \
      "${GLIBC_FLOOR_FAMILIES:-<unset>}" \
      "${GLIBC_FLOOR_MANYLINUX:-<unset>}" \
      "${GLIBC_FLOOR_TEGRA:-<unset>}" )
}

CONTRACT_VALUES="$(read_contract)"
if [ "$CONTRACT_VALUES" = '2.28|manylinux tegra|2.28|2.35' ]; then
  pass "contract declares 2.28 for manylinux and 2.35 for tegra, GLIBC_FLOOR unchanged at 2.28"
else
  fail "contract values: expected '2.28|manylinux tegra|2.28|2.35', got '$CONTRACT_VALUES'"
fi

resolve_family() {
  # shellcheck disable=SC1090
  ( set -euo pipefail
    . "$CONTRACT"
    glibc_floor_for_family "$1" )
}

for pair in 'manylinux:2.28' 'tegra:2.35'; do
  FAMILY="${pair%%:*}"
  EXPECTED_FLOOR="${pair##*:}"
  set +e
  RESOLVED="$(resolve_family "$FAMILY" 2>/dev/null)"
  RC_RESOLVE=$?
  set -e
  if [ "$RC_RESOLVE" -eq 0 ] && [ "$RESOLVED" = "$EXPECTED_FLOOR" ]; then
    pass "family $FAMILY resolves to floor $EXPECTED_FLOOR"
  else
    fail "family $FAMILY: expected floor $EXPECTED_FLOOR at rc=0, got rc=$RC_RESOLVE '$RESOLVED'"
  fi
done

set +e
UNKNOWN_OUT="$(resolve_family sbsa 2>&1)"
RC_UNKNOWN=$?
set -e
if [ "$RC_UNKNOWN" -ne 0 ] && [[ "$UNKNOWN_OUT" == *"sbsa"* ]]; then
  pass "an undeclared artifact family fails closed rather than yielding an empty floor"
else
  fail "unknown family: expected non-zero naming sbsa, got rc=$RC_UNKNOWN out=$UNKNOWN_OUT"
fi

printf 'test_check_glibc_floor: all cases passed\n'
