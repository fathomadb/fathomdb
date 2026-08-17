#!/usr/bin/env bash
# Exercise the real Engine::search AC-013 emitter, never the synthetic matrix runner.
set -euo pipefail

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

run_treatment() {
  local treatment="$1"
  local log="$tmp/$treatment.log"
  AGENT_LONG=1 AC013_VECTOR_DIM=384 AC013_CORPUS_N=10000 \
    AC013_SCALE_TREATMENT="$treatment" \
    cargo test --release -p fathomdb-engine --test perf_gates \
      ac_013_vector_retrieval_latency -- --exact --nocapture >"$log" 2>&1
  python3 - "$treatment" "$log" <<'PY'
import pathlib
import sys

treatment, log_path = sys.argv[1:]
lines = [line for line in pathlib.Path(log_path).read_text(encoding="utf-8").splitlines()
         if line.startswith("AC013_TREATMENT_RECORD ")]
assert len(lines) == 1, lines
parts = lines[0].split()[1:]
assert all("=" in part for part in parts), parts
fields = dict(part.split("=", 1) for part in parts)
expected = [
    "treatment", "n", "seed_write_ms", "embedding_ms", "projection_drain_ms",
    "accepted_writes", "vector_rows_after_drain", "drain_outcome", "samples_us",
    "result_counts", "query_errors", "query_timeouts", "query_skips",
    "query_invariant_failures",
]
assert [part.split("=", 1)[0] for part in parts] == expected, parts
assert fields["treatment"] == treatment, fields
assert fields["n"] == "10000", fields
for counter in ("query_errors", "query_timeouts", "query_skips", "query_invariant_failures"):
    assert fields[counter].isdecimal() and int(fields[counter]) == 0, fields
sample_count = 1 if treatment == "process_cold" else 1000
for field in ("samples_us", "result_counts"):
    values = fields[field].split(",")
    assert len(values) == sample_count, (field, len(values), sample_count)
    assert all(value.isdecimal() for value in values), (field, values[:3])
PY
}

run_treatment process_cold
run_treatment warm
