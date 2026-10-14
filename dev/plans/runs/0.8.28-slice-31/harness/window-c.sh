#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format diagnostics
# 0.8.28 Slice 31 Orin window C (test-plan 4.2, 4.3): G9 (AC31-13) with
# QUAL_QUIET_STRICT=1 and NO_SWAP=1, new (tip) vs base (slice base) vs Snew.
# Run last, with no cargo or rustc process on the Orin (Slice 30 deviation
# D-4): the strict quiet check refuses a run while one is present. Holds
# /tmp/fathomdb-gpu.lock for the whole window; inner series use a private
# lock ($GPU_LOCK). ORIN_GUARD_LOG gets a guard line around every process.
# If a Node CI upper bound exceeds 1.10, rerun once with N=40 in a new
# directory (G9_NODE_DIR=node2 G9_PARTS=node N=20 adds 20 more blocks).
# Env (defaults are the window A artifact paths): W, NODE_PROD, NODE_BASE,
# PY_W1, PY_W2, OUT, N (blocks, default 20), G9_PARTS, G9_NODE_DIR.
# Usage (background): nohup bash window-c.sh >window-c.log 2>&1 &
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
W=${W:-$HOME/fathomdb-slice31}
OUT=${OUT:-$W/window-c}
export SCRATCH=$OUT NODE_PROD=${NODE_PROD:-$W/node-tip} NODE_BASE=${NODE_BASE:-$W/node-base}
export PY_W1=${PY_W1:-$W/venv-tip/bin/python} PY_W2=${PY_W2:-$W/venv-tip/bin/python}
export QUAL_OUT=$OUT/series ORIN_GUARD_LOG=$OUT/guard.log GPU_LOCK=$OUT/series.lock
export QUAL_QUIET_STRICT=1 G9_STRICT=1 NO_SWAP=1
mkdir -p "$OUT/summaries"
for need in "$NODE_PROD/node_modules/fathomdb/dist/index.js" "$NODE_BASE/node_modules/fathomdb/dist/index.js" "$PY_W2"; do
  if [ ! -e "$need" ]; then echo "missing artifact $need" >&2; exit 2; fi
done
if pgrep -x cargo >/dev/null || pgrep -x rustc >/dev/null; then
  echo "cargo or rustc is running; window C needs a quiet host" >&2
  exit 4
fi
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
exec 9>/tmp/fathomdb-gpu.lock
echo "window C: waiting for /tmp/fathomdb-gpu.lock $(date -u +%T)"
flock 9
echo "window C: lock held $(date -u +%T)"
orin_guard_line window-c-start || exit 3
bash "$here/run-gates.sh" g9 "${N:-}" 9>&-
rc=$?
echo "=== g9 rc=$rc $(date -u +%T)"
python3 "$here/analyze.py" "$QUAL_OUT" g9 >"$OUT/summaries/g9.txt" 2>&1
orin_guard_line window-c-end || exit 3
echo "WINDOW-C-DONE rc=$rc $(date -u +%T); summary $OUT/summaries/g9.txt"
exit "$rc"
