#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format diagnostics
# 0.8.28 Slice 30 qualification: randomised interleaved blocks for timing
# comparisons. Each block runs one process of every cell, in an order shuffled
# with a recorded seed; blocks repeat <blocks> times. A throwaway warm-cache
# run of the first cell precedes block 1.
# Usage (GPU lock held by the caller): interleave.sh <cells.tsv> <blocks> <seed> <outdir>
# cells.tsv, tab-free, space-separated: "<label> <lang> <node-version> <module-or-python> <artifact> <KEY=VALUE,...|->"
#   lang node: <module-or-python> is the consumer dir (its node_modules/fathomdb/dist/index.js is the module);
#   lang py: <module-or-python> is the venv interpreter.
#   The KEY=VALUE list is exported for that cell only.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
cells=$1
blocks=$2
seed=$3
outdir=$4
mkdir -p "$outdir"
grep -v '^#' "$cells" | grep . >"$outdir/cells.tsv"
echo "seed=$seed blocks=$blocks utc_start=$(date -u +%FT%TZ)" >"$outdir/blocks.txt"
run_cell() { # label lang node target artifact env-list run-from
  local label=$1 lang=$2 nodever=$3 target=$4 artifact=$5 envs=$6 from=$7 kv
  (
    if [ "$envs" != - ]; then
      IFS=, read -ra kv <<<"$envs"
      for x in "${kv[@]}"; do export "${x?}"; done
    fi
    export CONSUMER_LANG=$lang NODE_VERSION=$nodever ARTIFACT=$artifact
    if [ "$lang" = py ]; then
      export PYTHON=$target
    else
      export FATHOMDB_MODULE="$target/node_modules/fathomdb/dist/index.js"
    fi
    RUN_FROM=$from bash "$here/run-series.sh" "$label" 1 "$outdir/$label"
  )
}
read -r l g n c a e <"$outdir/cells.tsv"
run_cell "$l" "$g" "$n" "$c" "$a" "$e" 0
for b in $(seq 1 "$blocks"); do
  # Python's seeded generator shuffles each block (a `shuf` random source of
  # repeated "seed-b" text gave every block the same order).
  order=$(python3 -c 'import random, sys; rows = open(sys.argv[1]).read().splitlines(); random.Random(sys.argv[2]).shuffle(rows); print("\n".join(rows))' "$outdir/cells.tsv" "$seed-$b")
  echo "block=$b order=$(cut -d' ' -f1 <<<"$order" | tr '\n' ' ')" >>"$outdir/blocks.txt"
  while read -r l g n c a e; do
    run_cell "$l" "$g" "$n" "$c" "$a" "$e" "$b" || exit $?
  done <<<"$order"
done
echo "utc_end=$(date -u +%FT%TZ)" >>"$outdir/blocks.txt"
