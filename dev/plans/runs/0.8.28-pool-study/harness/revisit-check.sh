#!/usr/bin/env bash
# 0.8.28 pool study, protocol section 10 item 5 (Phase 0): does the Slice 110
# failure still reproduce on this L4T/driver? Runs minimal_repro (three pages
# -> cuMemAllocAsync OOM expected) and its control alternately, then
# pool_teardown in the free-gap layout (explicit=keep) and its
# fill-before-pool control. Each run follows the host-quiet check.
# Usage (GPU lock held by the caller): revisit-check.sh <bindir> <outdir> [count]
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-host.sh
source "$here/lib-host.sh"
bin=$1; outdir=$2; count=${3:-20}
freegap=0x800000000,0xe00000000,0x1400000000,0x1a00000000,0x1f40001000
mkdir -p "$outdir"
series_header "$outdir" "probes=minimal_repro,pool_teardown" "count=$count"
run() { # name, then the command
  local name=$1; shift
  host_quiet_wait "$STUDY_QUIET_WAIT_S" 2>>"$outdir/waits.txt" || exit 4
  host_json true >"$outdir/$name.host.json"
  timeout "$STUDY_C_TIMEOUT_S" "$@" >"$outdir/$name.log" 2>&1
  echo "EXIT rc=$?" >>"$outdir/$name.log"
}
for i in $(seq -w 1 "$count"); do
  run "minimal-$i" "$bin/minimal_repro"
  run "minimal-control-$i" "$bin/minimal_repro" control
done
for i in $(seq -w 1 $((count / 2))); do
  run "teardown-freegap-keep-$i" "$bin/pool_teardown" primary "$freegap" explicit=keep
  run "teardown-freegap-fillbeforepool-$i" "$bin/pool_teardown" primary "$freegap" explicit=none fill-before-pool
done
{
  echo "minimal (3 pages): cuMemAllocAsync results"
  grep -h 'cuMemAllocAsync' "$outdir"/minimal-[0-9]*.log | sort | uniq -c
  echo "minimal control: cuMemAllocAsync results"
  grep -h 'cuMemAllocAsync' "$outdir"/minimal-control-*.log | sort | uniq -c
  echo "pool_teardown free-gap explicit=keep: RESULT pool_before/pool_after/async_after"
  grep -ho 'pool_before=[A-Z_]* async_before=[A-Z_]*\|pool_after=[A-Z_]*\|async_after=[A-Z_]*\|explicit_after=[A-Za-z_]*' "$outdir"/teardown-freegap-keep-*.log | sort | uniq -c
  echo "pool_teardown fill-before-pool control"
  grep -ho 'pool_before=[A-Z_]*\|async_before=[A-Z_]*' "$outdir"/teardown-freegap-fillbeforepool-*.log | sort | uniq -c
} >"$outdir/summary.txt"
cat "$outdir/summary.txt"
