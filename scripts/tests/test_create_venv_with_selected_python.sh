#!/usr/bin/env bash
# Regression tests for create_venv_with_selected_python (scripts/lib/agent-python-env.sh).
#
# 0.8.23 Slice 80.3: bootstrap.sh must create .venv with a real Python >=3.11
# interpreter (select_python_for_venv), and must fail closed — no partial
# .venv left behind — if none is available, rather than silently falling
# back to whatever bare `python3` happens to be.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../lib/agent-python-env.sh
. "$REPO_ROOT/scripts/lib/agent-python-env.sh"

FIX="$(mktemp -d)"
trap 'rm -rf "$FIX"' EXIT

fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }
pass() { printf 'PASS  %s\n' "$1"; }

# --- Arm A: on this real host, venv creation succeeds and the resulting
# interpreter really is >=3.11 (not the system 3.10 default).
VENV_A="$FIX/venv-a"
if ! create_venv_with_selected_python "$VENV_A" >"$FIX/out-a" 2>&1; then
  OUT_A_ERR="$(cat "$FIX/out-a")"
  fail "arm A: create_venv_with_selected_python failed on the real host: $OUT_A_ERR"
fi
[ -x "$VENV_A/bin/python3" ] || fail "arm A: no python3 in the created venv"
VER_A="$("$VENV_A/bin/python3" -c 'import sys; print(f"{sys.version_info[0]}.{sys.version_info[1]}")')"
case "$VER_A" in
  3.11|3.12|3.13) pass "arm A: created venv uses Python $VER_A (>=3.11, not the system 3.10 default)" ;;
  *) fail "arm A: created venv uses Python $VER_A, expected 3.11/3.12/3.13" ;;
esac

# --- Arm B: no qualifying interpreter anywhere -> fails closed, no partial
# .venv directory left on disk (a half-created venv is worse than none: it
# looks bootstrapped but silently carries the wrong interpreter).
EMPTY_PATH_DIR="$FIX/empty-path"; mkdir -p "$EMPTY_PATH_DIR"
VENV_B="$FIX/venv-b"
set +e
OUT_B="$(PATH="$EMPTY_PATH_DIR" create_venv_with_selected_python "$VENV_B" 2>&1)"
RC_B=$?
set -e
[ "$RC_B" -ne 0 ] || fail "arm B: expected failure with an empty PATH, got success"
if [ -e "$VENV_B" ]; then
  VENV_B_LISTING="$(ls -la "$VENV_B" 2>&1)"
  fail "arm B: a partial .venv was left behind after a failed selection: $VENV_B_LISTING"
fi
grep -Fq '3.11' <<<"$OUT_B" || fail "arm B: failure output did not name the minimum version: $OUT_B"
pass "arm B: fails closed with no partial venv when no interpreter qualifies"

# --- Arm C: bootstrap.sh actually wires the venv creation through this
# function, not a bare `python3 -m venv` that would reintroduce the too-old
# system-Python defect this sub-slice exists to fix.
BOOTSTRAP="$REPO_ROOT/scripts/bootstrap.sh"
grep -Fq 'create_venv_with_selected_python' "$BOOTSTRAP" \
  || fail "arm C: bootstrap.sh no longer calls create_venv_with_selected_python"
if grep -Eq '^\s*python3\s+-m\s+venv\b' "$BOOTSTRAP"; then
  fail "arm C: bootstrap.sh has a bare 'python3 -m venv' call again — this reintroduces the too-old-interpreter defect"
fi
pass "arm C: bootstrap.sh wires venv creation through create_venv_with_selected_python, not a bare python3"

# --- Arm D: interpreter selection succeeds, but the venv creation itself
# fails partway (here: an unwritable parent directory) -> still no partial
# target_dir left behind. This is distinct from arm B (selection fails
# before any venv work starts); here `-m venv` itself is what fails.
READONLY_PARENT="$FIX/readonly-parent"; mkdir -p "$READONLY_PARENT"
chmod 0555 "$READONLY_PARENT"
VENV_D="$READONLY_PARENT/venv-d"
set +e
create_venv_with_selected_python "$VENV_D" >/dev/null 2>&1
RC_D=$?
set -e
chmod 0755 "$READONLY_PARENT"  # restore before cleanup / the [ -e ] check below
[ "$RC_D" -ne 0 ] || fail "arm D: expected failure creating a venv under an unwritable parent, got success"
[ ! -e "$VENV_D" ] || fail "arm D: a partial venv directory was left behind after 'python -m venv' itself failed"
pass "arm D: no partial venv left behind when interpreter selection succeeds but 'python -m venv' itself fails"

echo "create_venv_with_selected_python: all arms passed"
