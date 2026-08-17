#!/usr/bin/env bash
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
runner="$root/scripts/perf-experiments/run-ac013.sh"
matrix="$root/scripts/perf-experiments/run-scale-ac013-matrix.sh"
test -x "$runner"
test -x "$matrix"
grep -q 'AC013_TREATMENT_RECORD' "$runner"
grep -q 'sha256sum.*-c' "$matrix"
grep -q 'vector_rows_after_drain' "$root/src/rust/crates/fathomdb-engine/tests/perf_gates.rs"

fakebin="$(mktemp -d)"
trap 'rm -rf "$fakebin"' EXIT
cat >"$fakebin/cargo" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *--no-run* ]]; then exit 0; fi
printf 'AC013_TREATMENT_RECORD treatment=%s n=1 seed_write_ms=1 embedding_ms=0 projection_drain_ms=1 accepted_writes=1 vector_rows_after_drain=1 drain_outcome=ok samples_us=7 result_counts=1\n' "${FAKE_TREATMENT:-process_cold}"
EOF
chmod +x "$fakebin/cargo"
log="$fakebin/record.log"
PATH="$fakebin:$PATH" FAKE_TREATMENT=warm AC013_SCALE_TREATMENT=warm LOG_PATH="$log" bash "$runner"
if PATH="$fakebin:$PATH" FAKE_TREATMENT=process_cold AC013_SCALE_TREATMENT=warm LOG_PATH="$log" bash "$runner"; then
  echo 'wrong treatment record must fail' >&2
  exit 1
fi
printf 'original\n' >"$fakebin/raw.log"
sha256sum "$fakebin/raw.log" >"$fakebin/raw.log.sha256"
sha256sum -c "$fakebin/raw.log.sha256"
printf 'tampered\n' >"$fakebin/raw.log"
if sha256sum -c "$fakebin/raw.log.sha256"; then
  echo 'tampered raw log must fail SHA verification' >&2
  exit 1
fi
