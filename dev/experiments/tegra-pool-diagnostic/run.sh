#!/usr/bin/env bash
# Run the unchanged installed Node row once; retain its original exit and diagnostics.
set -euo pipefail
if [ "$#" -ne 2 ]; then
  printf 'usage: %s INSTALLED_CONSUMER NEW_EVIDENCE_DIR\n' "$0" >&2
  exit 2
fi
consumer="$(cd "$1" && pwd -P)"
mkdir "$2"
evidence="$(cd "$2" && pwd -P)"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$script_dir/../../.." && pwd)"
row="$consumer/fathomdb-pool-diagnostic-row.mjs"
cp "$repo/scripts/release/smoke/tegra_node_package_row.mjs" "$row"
mkdir "$evidence/db"
sha256sum "$row" > "$evidence/row-source.sha256"
printf 'cwd=%q\n' "$consumer" > "$evidence/command.txt"
printf 'FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_GPU_ALLOCATION_WITNESS=1 FATHOMDB_GPU_POOL_DIAGNOSTIC=1 node %q diagnostic embed cuda:0 %q %q\n' \
  "$row" "$evidence/row.json" "$evidence/db" >> "$evidence/command.txt"
set +e
(cd "$consumer" && exec env FATHOMDB_EMBED_DEVICE=cuda:0 \
  FATHOMDB_GPU_ALLOCATION_WITNESS=1 FATHOMDB_GPU_POOL_DIAGNOSTIC=1 \
  node "$row" diagnostic embed cuda:0 "$evidence/row.json" "$evidence/db") \
  > "$evidence/stdout.log" 2> "$evidence/stderr.log" &
pid=$!
interrupt_signal=
finish_owned_child() {
  if [ -n "$interrupt_signal" ]; then
    trap '' INT TERM
  fi
  if [ -n "$interrupt_signal" ] && kill -0 "$pid" 2>/dev/null; then
    if ! kill -s "$interrupt_signal" "$pid" 2>/dev/null; then
      printf 'owned consumer exited before signal delivery\n' >&2
    fi
  fi
  wait "$pid"
  local status=$?
  printf '%s\n' "$status" > "$evidence/exit.txt"
  exit "$status"
}
trap 'interrupt_signal=INT; finish_owned_child' INT
trap 'interrupt_signal=TERM; finish_owned_child' TERM
printf '%s\n' "$pid" > "$evidence/pid.txt"
finish_owned_child
