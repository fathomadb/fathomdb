#!/usr/bin/env bash
# 0.8.28 pool study: randomised interleaved blocks for timing comparisons
# (protocol section 4.4). Each block runs one process of every cell, in an
# order shuffled with a recorded seed; blocks repeat <blocks> times. A
# throwaway warm-cache run of the first cell precedes block 1.
# Usage (GPU lock held by the caller): interleave.sh <cells.tsv> <blocks> <seed> <outdir>
# cells.tsv: "<label> <variant> <node-version> <consumer-dir> <KEY=VALUE,...|->"
#   per line; the consumer dir's node_modules/fathomdb/dist/index.js is the
#   module, its platform .node the artifact; the KEY=VALUE list is exported
#   for that cell only.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Fixes the series swap baseline once for every block (lib-host.sh exports it).
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
cells=$1; blocks=$2; seed=$3; outdir=$4
mkdir -p "$outdir"
grep -v '^#' "$cells" | grep . >"$outdir/cells.tsv"
echo "seed=$seed blocks=$blocks utc_start=$(date -u +%FT%TZ)" >"$outdir/blocks.txt"
run_cell() { # label variant node consumer env-list run-from count
  local label=$1 variant=$2 nodever=$3 consumer=$4 envs=$5 from=$6
  (
    if [ "$envs" != - ]; then IFS=, read -ra kv <<<"$envs"; for x in "${kv[@]}"; do export "${x?}"; done; fi
    export FATHOMDB_MODULE="$consumer/node_modules/fathomdb/dist/index.js"
    export ARTIFACT="$consumer/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node"
    RUN_FROM=$from bash "$here/run-series.sh" "$variant" "$nodever" 1 "$outdir/$label"
  )
}
read -r l v n c e <"$outdir/cells.tsv"
run_cell "$l" "$v" "$n" "$c" "$e" 0
for b in $(seq 1 "$blocks"); do
  # A shuf random source of repeated "seed-b" text gave every block the
  # same order (the seed prefix is all shuf reads), so each block is
  # shuffled here with Python's seeded generator instead.
  order=$(python3 -c 'import random, sys; rows = open(sys.argv[1]).read().splitlines(); random.Random(sys.argv[2]).shuffle(rows); print("\n".join(rows))' "$outdir/cells.tsv" "$seed-$b")
  echo "block=$b order=$(cut -d' ' -f1 <<<"$order" | tr '\n' ' ')" >>"$outdir/blocks.txt"
  while read -r l v n c e; do
    run_cell "$l" "$v" "$n" "$c" "$e" "$b" || exit $?
  done <<<"$order"
done
echo "utc_end=$(date -u +%FT%TZ)" >>"$outdir/blocks.txt"
