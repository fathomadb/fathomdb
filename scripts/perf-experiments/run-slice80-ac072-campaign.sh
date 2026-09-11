#!/usr/bin/env bash
# Dispatch the one sealed three-cell Slice 80.n AC-072 campaign, stopping closed.
set -euo pipefail

if [ "$#" -ne 10 ]; then
  echo "usage: $0 WORKTREE BINARY RAW_ROOT SOURCE_SHA BINARY_SHA256 INPUT_SHA256 RUNNER_SHA256 SCANNER_SHA256 VALIDATOR_SHA256 DISPATCHER_SHA256" >&2
  exit 2
fi

worktree="$1"
binary="$2"
raw_root="$3"
source_sha="$4"
binary_sha="$5"
input_sha="$6"
expected_runner_sha="$7"
expected_scanner_sha="$8"
expected_validator_sha="$9"
expected_dispatcher_sha="${10}"
root=$(cd "$(dirname "$0")/../.." && pwd)
runner="${SLICE80_AC072_RUNNER:-$root/scripts/perf-experiments/run-slice80-ac072-cell.sh}"
validator="${SLICE80_AC072_VALIDATOR:-$root/dev/tools/slice80_ac072_acceptance.py}"

actual_runner_sha=$(sha256sum "$runner" | awk '{print $1}')
actual_scanner_sha=$(sha256sum "$root/dev/tools/slice80_read_acceptance.py" | awk '{print $1}')
actual_validator_sha=$(sha256sum "$validator" | awk '{print $1}')
actual_dispatcher_sha=$(sha256sum "$root/scripts/perf-experiments/run-slice80-ac072-campaign.sh" | awk '{print $1}')
if [ "$actual_runner_sha" != "$expected_runner_sha" ] || [ "$actual_scanner_sha" != "$expected_scanner_sha" ] || \
  [ "$actual_validator_sha" != "$expected_validator_sha" ] || [ "$actual_dispatcher_sha" != "$expected_dispatcher_sha" ]; then
  echo "Slice 80 AC-072 runner/validator identity drift" >&2
  exit 2
fi

if ! mkdir "$raw_root"; then
  echo "campaign root must be new and create-only: $raw_root" >&2
  exit 2
fi

for label in R1 R2 R3; do
  raw_log="$raw_root/$label.log"
  verdict="$raw_root/$label.verdict.json"
  set +e
  timeout --kill-after=15s 3600 "$runner" "$worktree" "$binary" "$raw_log" "$source_sha" "$binary_sha" "$input_sha" acceptance 10000 "$expected_runner_sha" "$expected_scanner_sha"
  runner_status=$?
  set -e
  if ! PYTHONPATH="$root${PYTHONPATH:+:$PYTHONPATH}" "$validator" validate-cell --log "$raw_log" --label "$label" --purpose acceptance >"$verdict"; then
    exit 1
  fi
  if [ "$runner_status" -ne 0 ]; then
    exit 1
  fi
done
PYTHONPATH="$root${PYTHONPATH:+:$PYTHONPATH}" "$validator" summarize-campaign --logs "$raw_root/R1.log" "$raw_root/R2.log" "$raw_root/R3.log" >"$raw_root/summary.json"
