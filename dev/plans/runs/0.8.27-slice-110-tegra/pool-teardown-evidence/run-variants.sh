#!/usr/bin/env bash
# Slice 110 early-cuInit round: the teardown series without the confound of a
# live explicit pool, plus a control taken after cuInit. Runs pool_teardown in
# fresh processes, one at a time:
#   - <count> runs each of primary/nonprimary x open/free-gap with
#     explicit=none (no explicit pool is ever created);
#   - <count> runs each of primary/nonprimary in the free-gap layout with
#     explicit=destroy (created, used, destroyed before teardown);
#   - <count> runs of primary in the free-gap layout with fill-before-pool
#     and explicit=none (every window hole blocked after cuInit and before the
#     first default-pool query).
# Callers wrap the whole series in `flock <scratch>/gpu.lock`.
# Usage: run-variants.sh <pool_teardown binary> <outdir> [count]
set -euo pipefail
bin=$1
outdir=$2
count=${3:-10}
freegap=0x800000000,0xe00000000,0x1400000000,0x1a00000000,0x1f40001000
mkdir -p "$outdir"
for mode in primary nonprimary; do
  for i in $(seq -w 1 "$count"); do
    "$bin" "$mode" - explicit=none >"$outdir/$mode-open-none-$i.log" 2>&1
    "$bin" "$mode" "$freegap" explicit=none >"$outdir/$mode-freegap-none-$i.log" 2>&1
    "$bin" "$mode" "$freegap" explicit=destroy >"$outdir/$mode-freegap-destroy-$i.log" 2>&1
  done
done
for i in $(seq -w 1 "$count"); do
  "$bin" primary "$freegap" explicit=none fill-before-pool \
    >"$outdir/primary-freegap-fillbeforepool-$i.log" 2>&1
done
