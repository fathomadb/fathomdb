#!/usr/bin/env bash
# RED-first coverage for the glibc-floor doc-truth gate (AC80-9, AC80-26).
#
# docs/compatibility/index.md must state the same floor
# scripts/release/glibc-floor-contract.sh declares, PER ARTIFACT FAMILY. A
# fixture doc asserting a false floor for EITHER family must fail the gate.
#
# 0.8.23 Slice 80.6 (D-80.6-5): the pre-80.6 gate read the doc with
# `grep -m1` — first match only — against a single $GLIBC_FLOOR, so the moment
# the doc carried a second claim a wrong Tegra floor was never checked and
# passed silently. Case 2 below is that exact drift and is the load-bearing
# case here; it passed the gate before the restructure.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-glibc-floor-doc-truth.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

# --- Case 1: the real repo doc matches the real contract -> gate PASSes ---
set +e
OUT1="$("$CHECKER" 2>&1)"
RC1=$?
set -e
if [ "$RC1" -eq 0 ]; then
  pass "real doc matches real contract"
else
  fail "real doc: expected exit 0, got rc=$RC1 out=$OUT1"
fi

# --- Case 2 (AC80-26, load-bearing): a doc whose FIRST claim is correct and
# whose SECOND claim is wrong must FAIL. Written in the pre-80.6 unmarkered
# form on purpose: that is the shape the old `grep -m1` gate accepted, and it
# accepted it with exit 0 — the silent drift this case exists to forbid.
LEGACY_SECOND_CLAIM_DOC="$WORK/legacy-second-claim.md"
cat > "$LEGACY_SECOND_CLAIM_DOC" <<'EOF'
# Compatibility

**Measured glibc floor: 2.28**, for the Python wheel and the npm platform
binary, on both architectures.

**Measured glibc floor: 2.99**, for the Jetson/Tegra CUDA artifact.
EOF
set +e
OUT2="$("$CHECKER" --doc "$LEGACY_SECOND_CLAIM_DOC" 2>&1)"
RC2=$?
set -e
if [ "$RC2" -eq 1 ]; then
  pass "a correct first claim does not excuse a wrong second claim"
else
  fail "wrong-second-claim fixture: expected exit 1, got rc=$RC2 out=$OUT2"
fi

# --- Case 3: markered doc, correct manylinux floor, WRONG tegra floor ->
# gate FAILs and names the family and the wrong value. ---
WRONG_TEGRA_DOC="$WORK/wrong-tegra.md"
cat > "$WRONG_TEGRA_DOC" <<'EOF'
# Compatibility

**Measured glibc floor (manylinux): 2.28**, for the Python wheel and the npm
platform binary, on both architectures.

**Measured glibc floor (tegra): 2.99**, for the Jetson/Tegra CUDA artifact.
EOF
set +e
OUT3="$("$CHECKER" --doc "$WRONG_TEGRA_DOC" 2>&1)"
RC3=$?
set -e
if [ "$RC3" -eq 1 ] && [[ "$OUT3" == *"tegra"* ]] && [[ "$OUT3" == *"2.99"* ]]; then
  pass "wrong tegra floor fails, names the family and the mismatch"
else
  fail "wrong-tegra fixture: expected exit 1 naming tegra and 2.99, got rc=$RC3 out=$OUT3"
fi

# --- Case 4: markered doc, WRONG manylinux floor, correct tegra floor ---
WRONG_MANYLINUX_DOC="$WORK/wrong-manylinux.md"
cat > "$WRONG_MANYLINUX_DOC" <<'EOF'
# Compatibility

**Measured glibc floor (manylinux): 2.17**, for the Python wheel and the npm
platform binary, on both architectures.

**Measured glibc floor (tegra): 2.35**, for the Jetson/Tegra CUDA artifact.
EOF
set +e
OUT4="$("$CHECKER" --doc "$WRONG_MANYLINUX_DOC" 2>&1)"
RC4=$?
set -e
if [ "$RC4" -eq 1 ] && [[ "$OUT4" == *"manylinux"* ]] && [[ "$OUT4" == *"2.17"* ]]; then
  pass "wrong manylinux floor fails, names the family and the mismatch"
else
  fail "wrong-manylinux fixture: expected exit 1 naming manylinux and 2.17, got rc=$RC4 out=$OUT4"
fi

# --- Case 5: a doc that claims one family and silently omits the other ---
MISSING_FAMILY_DOC="$WORK/missing-family.md"
cat > "$MISSING_FAMILY_DOC" <<'EOF'
# Compatibility

**Measured glibc floor (manylinux): 2.28**, for the Python wheel and the npm
platform binary, on both architectures.
EOF
set +e
OUT5="$("$CHECKER" --doc "$MISSING_FAMILY_DOC" 2>&1)"
RC5=$?
set -e
if [ "$RC5" -eq 1 ] && [[ "$OUT5" == *"tegra"* ]]; then
  pass "a doc omitting a declared family fails closed, naming the family"
else
  fail "missing-family fixture: expected exit 1 naming tegra, got rc=$RC5 out=$OUT5"
fi

# --- Case 6: a fixture doc with no floor statement at all -> gate FAILs ---
NO_CLAIM_DOC="$WORK/no-claim.md"
cat > "$NO_CLAIM_DOC" <<'EOF'
# Compatibility

Nothing to see here.
EOF
set +e
OUT6="$("$CHECKER" --doc "$NO_CLAIM_DOC" 2>&1)"
RC6=$?
set -e
if [ "$RC6" -eq 1 ]; then
  pass "doc with no floor claim fails rather than silently passing"
else
  fail "no-claim doc: expected exit 1, got rc=$RC6 out=$OUT6"
fi

# --- Case 7: every declared family must be checked. A doc that repeats a
# family's marker with a second, contradictory value must FAIL — otherwise the
# per-family restructure would just move the `grep -m1` blind spot one level
# down. ---
CONTRADICTORY_DOC="$WORK/contradictory.md"
cat > "$CONTRADICTORY_DOC" <<'EOF'
# Compatibility

**Measured glibc floor (manylinux): 2.28**, for the Python wheel and the npm
platform binary, on both architectures.

**Measured glibc floor (tegra): 2.35**, for the Jetson/Tegra CUDA artifact.

Elsewhere, wrongly: Measured glibc floor (tegra): 2.31.
EOF
set +e
OUT7="$("$CHECKER" --doc "$CONTRADICTORY_DOC" 2>&1)"
RC7=$?
set -e
if [ "$RC7" -eq 1 ] && [[ "$OUT7" == *"2.31"* ]]; then
  pass "a contradictory repeat claim for a family fails, naming the bad value"
else
  fail "contradictory fixture: expected exit 1 naming 2.31, got rc=$RC7 out=$OUT7"
fi

printf 'test_check_glibc_floor_doc_truth: all cases passed\n'
