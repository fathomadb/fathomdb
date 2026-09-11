#!/usr/bin/env bash
# Prove both Slice 80 collectors have a benchmark-free production snapshot path.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)

for runner in \
  "$root/scripts/perf-experiments/run-slice80-ac081-cell.sh" \
  "$root/scripts/perf-experiments/run-slice80-ac072-cell.sh"; do
  grep -F 'SLICE80_COLLECTOR_ONLY' "$runner" >/dev/null
  grep -F 'snapshot start' "$runner" >/dev/null
  grep -F 'snapshot end' "$runner" >/dev/null
done

temp_dir=$(mktemp -d /tmp/fathomdb-slice80-readiness.XXXXXX)
competitor_pid=""
cleanup() {
  if [ -n "$competitor_pid" ]; then
    kill "$competitor_pid" 2>/dev/null || true
    wait "$competitor_pid" 2>/dev/null || true
  fi
  rm -rf "$temp_dir"
}
trap cleanup EXIT

source_sha=$(git -C "$root" rev-parse HEAD)
input_sha=$(git -C "$root" ls-tree -r "$source_sha" -- \
  Cargo.toml Cargo.lock .cargo/config.toml \
  src/rust/crates/fathomdb-engine/Cargo.toml \
  src/rust/crates/fathomdb-engine/src \
  src/rust/crates/fathomdb-engine/tests/perf_gates.rs \
  src/rust/crates/fathomdb-engine/tests/reader_pool.rs \
  src/rust/crates/fathomdb-query src/rust/crates/fathomdb-schema \
  src/rust/crates/fathomdb-embedder src/rust/crates/fathomdb-embedder-api | \
  sha256sum | awk '{print $1}')
true_sha=$(sha256sum /bin/true | awk '{print $1}')
ac072_runner_sha=$(sha256sum "$root/scripts/perf-experiments/run-slice80-ac072-cell.sh" | awk '{print $1}')
scanner_sha=$(sha256sum "$root/dev/tools/slice80_read_acceptance.py" | awk '{print $1}')

SLICE80_COLLECTOR_ONLY=1 "$root/scripts/perf-experiments/run-slice80-ac081-cell.sh" \
  "$root" /bin/true "$temp_dir/ac081.log" "$source_sha" "$true_sha" "$input_sha"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/dev/tools/slice80_read_acceptance.py" \
  verify-collector-readiness --log "$temp_dir/ac081.log" \
  --identity-prefix SLICE80_IDENTITY | rg '"environment_applicable": true' >/dev/null

SLICE80_COLLECTOR_ONLY=1 "$root/scripts/perf-experiments/run-slice80-ac072-cell.sh" \
  "$root" /bin/true "$temp_dir/ac072.log" "$source_sha" "$true_sha" "$input_sha" smoke 10 "$ac072_runner_sha" "$scanner_sha"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/dev/tools/slice80_read_acceptance.py" \
  verify-collector-readiness --log "$temp_dir/ac072.log" \
  --identity-prefix SLICE80_AC072_IDENTITY | rg '"environment_applicable": true' >/dev/null

bash -c 'while :; do sleep 1; done' "$root/scripts/perf-experiments/run-slice80-ac081-cell.sh" &
competitor_pid=$!
SLICE80_COLLECTOR_ONLY=1 "$root/scripts/perf-experiments/run-slice80-ac081-cell.sh" \
  "$root" /bin/true "$temp_dir/ac081-competing.log" "$source_sha" "$true_sha" "$input_sha"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/dev/tools/slice80_read_acceptance.py" \
  verify-collector-readiness --log "$temp_dir/ac081-competing.log" \
  --identity-prefix SLICE80_IDENTITY | rg '"environment_applicable": false' >/dev/null

echo "ok test_slice80_collector_readiness"
