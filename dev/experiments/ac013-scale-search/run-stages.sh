#!/usr/bin/env bash
# Runs the pre-registered stages (PREREG.md) through ssweep.py.
#
#   BIN=<prebuilt exp_ac013_scale_search binary> SHA=<its commit> \
#   OUT=<scratch out dir> LOCK=<measurement lock> LME=<dir from lme_corpus.py> \
#   dev/experiments/ac013-scale-search/run-stages.sh A [B ...]
#
# Stage C takes C_NS (default "100000 1000000"; amendment 1 ran 100000 and
# 300000) and C_ARGS (extra ssweep flags). Stage E needs GATE_BIN (a prebuilt
# perf_gates test binary) and E_BEST.
set -euo pipefail
D="$(cd "$(dirname "$0")" && pwd)"
: "${BIN:?}" "${SHA:?}" "${OUT:?}" "${LOCK:?}" "${LME:?}"
R=(--cells "$D/cells.json" --out "$OUT" --lock "$LOCK" --bin "$BIN" --bin-sha "$SHA")
sw() { python3 "$D/ssweep.py" run "${R[@]}" "$@"; }
syn_samples() { case "$1" in 100000) echo 300 ;; *) echo 1000 ;; esac; }

stage_A() {
  for n in 10000 30000 100000; do
    sw --only BASE,CS64 --corpus syn --n $n --samples "$(syn_samples $n)" \
      --passes hybrid,text,vector --trace
    sw --only BASE,CS64 --corpus lme --lme-dir "$LME" --n $n --samples 500 \
      --passes hybrid,text,vector --trace
  done
}

stage_B() {
  local cells=BASE,CS64,CS64+ELIG,CS64+FTS,CS64+ALL
  for n in 10000 100000 30000; do
    local reps=2
    [ $n = 30000 ] && reps=1
    sw --only $cells --corpus syn --n $n --samples "$(syn_samples $n)" --repeats $reps
    sw --only $cells --corpus lme --lme-dir "$LME" --n $n --samples 500 --repeats $reps
  done
}

stage_Bt() {
  for n in 10000 100000; do
    sw --only CS64+ELIG,CS64+FTS,CS64+ALL --corpus syn --n $n --samples "$(syn_samples $n)" \
      --trace
  done
}

stage_C() {
  for n in ${C_NS:-100000 1000000}; do
    sw --only BASE+ELIG,CS64+ELIG --corpus syn --n $n --samples 200 --passes vector --trace ${C_ARGS:-}
    sw --only BASE+ELIG,CS64+ELIG --corpus syn --n $n --samples 200 --passes vector ${C_ARGS:-}
  done
}

stage_D() {
  sw --only CS64 --corpus lme --lme-dir "$LME" --n 10000 --samples 500 --passes hybrid,vector \
    --supersede 10
}

stage_E() {
  : "${GATE_BIN:?}"
  python3 "$D/ssweep.py" run-gate --cells "$D/cells.json" --out "$OUT" --lock "$LOCK" \
    --bin "$GATE_BIN" --bin-sha "$SHA" --n 10000 --repeats 2 --only "CS64,${E_BEST:?}"
}

for s in "$@"; do "stage_$s"; done
