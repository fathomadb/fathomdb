#!/usr/bin/env bash
# Slice 110 review fix 2: default-pool survival across context teardown.
# Runs pool_teardown in fresh processes, one at a time: <count> runs each of
# primary/nonprimary in the unobstructed and free-gap layouts, then <control>
# primary runs with the three standard blockers mapped before cuInit (a
# control that the probe does detect an unavailable pool). Callers wrap the
# whole series in `flock <scratch>/gpu.lock`.
# Usage: run-series.sh <pool_teardown binary> <outdir> [count] [control]
set -euo pipefail
bin=$1
outdir=$2
count=${3:-10}
control=${4:-5}
freegap=0x800000000,0xe00000000,0x1400000000,0x1a00000000,0x1f40001000
standard=0x980000000,0x1100000000,0x1880000000
mkdir -p "$outdir"
for mode in primary nonprimary; do
  for i in $(seq -w 1 "$count"); do
    "$bin" "$mode" >"$outdir/$mode-open-$i.log" 2>&1
    "$bin" "$mode" "$freegap" >"$outdir/$mode-freegap-$i.log" 2>&1
  done
done
for i in $(seq 1 "$control"); do
  "$bin" primary "$standard" >"$outdir/primary-control-std3-$i.log" 2>&1
done
