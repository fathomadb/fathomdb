#!/usr/bin/env bash
# The timed and traced runs behind results.json, in PREREG.md order.
#
#   S=<scratch>/ac013 dev/experiments/ac013-seed-drain/run-sweep.sh <step>
#
# Steps: timed (PREREG run 1), big (run 2, n=300k; pass ONLY=CS64-AL to drop
# CS64), trace (run 3), vreads (run 4's AC-013 search digests). Run from the
# repository root with CARGO_TARGET_DIR set as for the build. OUT defaults to
# $S/drain/sweep; LOCK to $S/measure.lock.
set -euo pipefail
step="${1:?step}"
D=dev/experiments/ac013-seed-drain
OUT="${OUT:-$S/drain/sweep}"
LOCK="${LOCK:-$S/measure.lock}"
R=(--cells "$D/cells.json" --out "$OUT" --lock "$LOCK")
case "$step" in
  timed)
    python3 "$D/drain_sweep.py" run "${R[@]}" --repeats 2 --n 10000,30000,100000 \
      --only BASE,BASE-c4000,BASE-c4000-AL,CS64,CS64-AL
    python3 "$D/drain_sweep.py" run "${R[@]}" --repeats 2 --n 10000,100000 --only CS64-o,CS64-AL-o
    python3 "$D/drain_sweep.py" run "${R[@]}" --repeats 2 --n 10000 --only CS64-t1,CS64-AL-t1
    ;;
  big)
    python3 "$D/drain_sweep.py" run "${R[@]}" --repeats 2 --n 300000 --only "${ONLY:-CS64,CS64-AL}"
    ;;
  trace)
    python3 "$D/drain_sweep.py" run "${R[@]}" --repeats 1 --n 10000,100000 --trace \
      --only BASE-c4000,CS64,CS64-AL
    python3 "$D/drain_sweep.py" run "${R[@]}" --repeats 1 --n 10000 --trace --only CS64-t1,CS64-AL-t1
    ;;
  vreads)
    mkdir -p "$OUT/vreads"
    for cell in CS64 CS64-AL; do
      extra=()
      [ "$cell" = CS64-AL ] && extra=(FATHOMDB_PERF_DRAIN_SCAN=armlimit)
      flock "$LOCK" env FATHOMDB_PERF_EXPERIMENTS=1 FATHOMDB_PERF_VEC0_CHUNK_SIZE=64 "${extra[@]}" \
        AC013_RESULTS_OUT="$OUT/vreads/$cell.hits" \
        cargo test --release -p fathomdb-engine --features test-hooks --test exp_ac013_vector_reads \
        -- --ignored --nocapture >"$OUT/vreads/$cell.log" 2>&1
      python3 -c 'import json,re,sys; d=json.loads(re.search(r"AC013_VREADS (\{.*\})", open(sys.argv[1]).read())[1]); print(sys.argv[2], d["results_digest"], d["ids_digest"])' \
        "$OUT/vreads/$cell.log" "$cell"
    done
    cmp "$OUT/vreads/CS64.hits" "$OUT/vreads/CS64-AL.hits" && echo "per-query hits identical"
    ;;
  *) echo "unknown step $step" >&2; exit 2 ;;
esac
