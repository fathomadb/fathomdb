#!/usr/bin/env bash
# RED-first coverage for the glibc-floor doc-truth gate (AC80-9).
#
# docs/compatibility/index.md must state the same floor
# scripts/release/glibc-floor-contract.sh declares. A fixture doc asserting
# a false floor must fail the gate.
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

# --- Case 2: a fixture doc asserting a false floor -> gate FAILs ---
FIXTURE_DOC="$WORK/index.md"
cat > "$FIXTURE_DOC" <<'EOF'
# Compatibility

**Measured glibc floor: 2.17**, for both the Python wheel and the npm
platform binary, on both architectures.
EOF
set +e
OUT2="$("$CHECKER" --doc "$FIXTURE_DOC" 2>&1)"
RC2=$?
set -e
if [ "$RC2" -eq 1 ] && [[ "$OUT2" == *"2.17"* ]]; then
  pass "false-floor fixture doc fails, names the mismatch"
else
  fail "false-floor fixture: expected exit 1 naming 2.17, got rc=$RC2 out=$OUT2"
fi

# --- Case 3: a fixture doc with no floor statement at all -> gate FAILs ---
NO_CLAIM_DOC="$WORK/no-claim.md"
cat > "$NO_CLAIM_DOC" <<'EOF'
# Compatibility

Nothing to see here.
EOF
set +e
OUT3="$("$CHECKER" --doc "$NO_CLAIM_DOC" 2>&1)"
RC3=$?
set -e
if [ "$RC3" -eq 1 ]; then
  pass "doc with no floor claim fails rather than silently passing"
else
  fail "no-claim doc: expected exit 1, got rc=$RC3 out=$OUT3"
fi

printf 'test_check_glibc_floor_doc_truth: all cases passed\n'
