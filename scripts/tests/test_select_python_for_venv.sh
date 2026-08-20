#!/usr/bin/env bash
# Regression tests for select_python_for_venv (scripts/lib/agent-python-env.sh).
#
# 0.8.23 Slice 80.3: bootstrap.sh must pick a real Python >=3.11 to create
# .venv, rather than bare `python3` (which is 3.10 on Ubuntu 22.04/L4T R36 —
# too old for stdlib `tomllib`, which several release/CI gates require). This
# test fixtures isolate PATH so their result never depends on which Python
# versions happen to be installed on the host. A selection function that only
# probes bare PATH commands silently fails on hosts whose modern interpreter is
# available only through `uv python find`; the fixtures encode that case.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../lib/agent-python-env.sh
. "$REPO_ROOT/scripts/lib/agent-python-env.sh"

FIX="$(mktemp -d)"
trap 'rm -rf "$FIX"' EXIT

fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }
pass() { printf 'PASS  %s\n' "$1"; }

# Build a fake interpreter shim that answers --version with the given version
# string, at $1/python$2 (e.g. mkshim "$dir" 3.12 -> $dir/python3.12).
mkshim() {
  local dir="$1" ver="$2"
  mkdir -p "$dir"
  cat >"$dir/python$ver" <<SHIM
#!/bin/bash
if [ "\$1" = "--version" ]; then
  echo "Python $ver.4"
  exit 0
fi
exit 1
SHIM
  chmod +x "$dir/python$ver"
}

# Build a fake `uv` that supports `uv python find <ver>`, resolving only the
# versions passed after the first two args (e.g. mkuv "$dir" "3.12:$target").
mkuv() {
  local dir="$1"
  shift
  mkdir -p "$dir"
  {
    echo '#!/bin/bash'
    # shellcheck disable=SC2016  # $1/$2/$3 must stay unexpanded — they're the
    # generated shim's OWN parameters, resolved when it runs later, not here.
    echo 'if [ "$1" = "python" ] && [ "$2" = "find" ]; then'
    # shellcheck disable=SC2016  # same: $3 belongs to the generated shim.
    echo '  case "$3" in'
    for mapping in "$@"; do
      local ver="${mapping%%:*}" target="${mapping#*:}"
      # An empty target means "this version does not resolve" — omit its case
      # arm entirely so it falls through to the default error branch below,
      # matching real `uv python find`'s exit-1-with-stderr-message behavior
      # on an unresolvable version (never "succeeds with empty output").
      [ -n "$target" ] || continue
      printf '    "%s") echo "%s"; exit 0 ;;\n' "$ver" "$target"
    done
    echo '  esac'
    # shellcheck disable=SC2016  # same: $3 belongs to the generated shim.
    echo '  echo "error: No interpreter found for Python $3" >&2'
    echo '  exit 1'
    echo 'fi'
    echo 'exit 1'
  } >"$dir/uv"
  chmod +x "$dir/uv"
}

# Keep the fixture PATH free of the host's `/usr/bin/python3.12` (or any
# future Python): every candidate must be supplied deliberately by the arm.
NO_SYSTEM_PATH="$FIX/no-system-path"; mkdir -p "$NO_SYSTEM_PATH"

# --- Arm A: a single bare python3.11 on PATH is selected -------------------
DIR_A="$FIX/a"; mkshim "$DIR_A" 3.11
OUT_A="$(PATH="$DIR_A:$NO_SYSTEM_PATH" select_python_for_venv)"
RC_A=$?
[ "$RC_A" -eq 0 ] || fail "arm A: expected success, got rc=$RC_A"
[ "$OUT_A" = "$DIR_A/python3.11" ] || fail "arm A: expected $DIR_A/python3.11, got $OUT_A"
pass "arm A: a single bare python3.11 on PATH is selected"

# --- Arm B: bare 3.11 and 3.12 both on PATH -> the higher version wins -----
DIR_B="$FIX/b"; mkshim "$DIR_B" 3.11; mkshim "$DIR_B" 3.12
OUT_B="$(PATH="$DIR_B:$NO_SYSTEM_PATH" select_python_for_venv)"
[ "$OUT_B" = "$DIR_B/python3.12" ] || fail "arm B: expected python3.12 preferred over 3.11, got $OUT_B"
pass "arm B: the highest available bare version wins (3.12 over 3.11)"

# --- Arm C: no bare 3.11+ on PATH, but `uv python find` resolves one -------
# This is the case this exact host is in: uv-managed pythons exist but are
# not bare PATH commands.
DIR_C="$FIX/c"; mkdir -p "$DIR_C"
UV_TARGET_C="$FIX/uv-managed/python3.12"
mkdir -p "$(dirname "$UV_TARGET_C")"; : >"$UV_TARGET_C"; chmod +x "$UV_TARGET_C"
mkuv "$DIR_C" "3.13:" "3.12:$UV_TARGET_C" "3.11:"
OUT_C="$(PATH="$DIR_C:$NO_SYSTEM_PATH" select_python_for_venv)"
[ "$OUT_C" = "$UV_TARGET_C" ] || fail "arm C: expected uv-resolved $UV_TARGET_C, got $OUT_C"
pass "arm C: falls back to 'uv python find' when no bare command exists (the real host case)"

# --- Arm D: bare 3.11 present AND uv resolves 3.13 -> version, not mechanism, wins
DIR_D="$FIX/d"; mkshim "$DIR_D" 3.11
UV_TARGET_D="$FIX/uv-managed-d/python3.13"
mkdir -p "$(dirname "$UV_TARGET_D")"; : >"$UV_TARGET_D"; chmod +x "$UV_TARGET_D"
mkuv "$DIR_D" "3.13:$UV_TARGET_D" "3.12:" "3.11:"
OUT_D="$(PATH="$DIR_D:$NO_SYSTEM_PATH" select_python_for_venv)"
[ "$OUT_D" = "$UV_TARGET_D" ] || fail "arm D: expected the higher uv-resolved 3.13 ($UV_TARGET_D) to beat bare 3.11, got $OUT_D"
pass "arm D: preference is by version across mechanisms, not bare-PATH-first"

# --- Arm E: nothing available anywhere -> fails closed with actionable message
DIR_E="$FIX/e"; mkdir -p "$DIR_E"
set +e
OUT_E="$(PATH="$DIR_E:$NO_SYSTEM_PATH" select_python_for_venv 2>"$FIX/err-e")"
RC_E=$?
set -e
ERR_E="$(cat "$FIX/err-e")"
[ "$RC_E" -ne 0 ] || fail "arm E: expected failure when nothing is available, got success ($OUT_E)"
[ -z "$OUT_E" ] || fail "arm E: expected no stdout on failure, got $OUT_E"
grep -Fq '3.11' <<<"$ERR_E" || fail "arm E: failure message did not name the minimum version (3.11): $ERR_E"
grep -Fqi 'uv python install' <<<"$ERR_E" || fail "arm E: failure message did not mention 'uv python install' as an install path: $ERR_E"
pass "arm E: fails closed with an actionable message when no 3.11+ interpreter exists anywhere"

# --- Arm F: a stale/broken uv on PATH (uv present, resolves nothing, no bare
# command either) still fails closed rather than crashing the caller.
DIR_F="$FIX/f"; mkdir -p "$DIR_F"
mkuv "$DIR_F" "3.13:" "3.12:" "3.11:"
set +e
OUT_F="$(PATH="$DIR_F:$NO_SYSTEM_PATH" select_python_for_venv 2>"$FIX/err-f")"
RC_F=$?
set -e
[ "$RC_F" -ne 0 ] || fail "arm F: expected failure when uv resolves nothing, got success ($OUT_F)"
pass "arm F: uv present but resolving nothing still fails closed, not a crash"

echo "select_python_for_venv: all arms passed"
