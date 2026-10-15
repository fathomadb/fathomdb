#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format diagnostics
# 0.8.28 Slice 31 Orin window B (test-plan 4.2): gates G1-G8 on the Orin
# artifacts built from the tip, one locked GPU session. Holds
# /tmp/fathomdb-gpu.lock for the whole window; the per-series locks of
# run-gates.sh use a private lock file inside the window ($GPU_LOCK), so they
# never wait on the window's own lock. ORIN_GUARD_LOG gets one guard line
# (L4T release, CUDA symlink) before and after every consumer process; a
# change stops the window.
# Env (defaults are the window A artifact paths): W (~/fathomdb-slice31),
# NODE_PROD, PY_W1, PY_W2, FDB_CLI, OUT (logs and series), GATES.
# Usage (background): nohup bash window-b.sh >window-b.log 2>&1 &
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
W=${W:-$HOME/fathomdb-slice31}
OUT=${OUT:-$W/window-b}
export SCRATCH=$OUT NODE_PROD=${NODE_PROD:-$W/node-tip}
export PY_W1=${PY_W1:-$W/venv-tip/bin/python} PY_W2=${PY_W2:-$W/venv-tip/bin/python}
export FDB_CLI=${FDB_CLI:-$W/artifacts/fathomdb}
export QUAL_OUT=$OUT/series ORIN_GUARD_LOG=$OUT/guard.log GPU_LOCK=$OUT/series.lock
export QUAL_QUIET_STRICT=${QUAL_QUIET_STRICT:-0}
mkdir -p "$OUT/summaries"
for need in "$NODE_PROD/node_modules/fathomdb/dist/index.js" "$PY_W1" "$PY_W2" "$FDB_CLI"; do
  if [ ! -e "$need" ]; then echo "missing artifact $need" >&2; exit 2; fi
done
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
exec 9>/tmp/fathomdb-gpu.lock
echo "window B: waiting for /tmp/fathomdb-gpu.lock $(date -u +%T)"
flock 9
echo "window B: lock held $(date -u +%T)"
orin_guard_line window-b-start || exit 3
rc_all=0
for gate in ${GATES:-g1 g2 g3 g3s g4 g5 g6 g7 g8 g8s}; do
  echo "=== $gate start $(date -u +%T)"
  bash "$here/run-gates.sh" "$gate" 9>&-
  rc=$?
  echo "=== $gate rc=$rc $(date -u +%T)"
  if [ "$rc" != 0 ]; then rc_all=1; fi
  orin_guard_line "after-gate-$gate" || exit 3
done
for gate in ${GATES:-g1 g2 g3 g3s g4 g5 g6 g7 g8 g8s}; do
  python3 "$here/analyze.py" "$QUAL_OUT" "$gate" >"$OUT/summaries/$gate.txt" 2>&1
done
orin_guard_line window-b-end || exit 3
echo "WINDOW-B-DONE rc=$rc_all $(date -u +%T); summaries in $OUT/summaries"
exit "$rc_all"
