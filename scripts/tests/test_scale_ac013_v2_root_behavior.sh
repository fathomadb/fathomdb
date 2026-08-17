#!/usr/bin/env bash
# Behavioral V2 root closure contract; fake runner only, never measurement.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fake="$tmp/fake"
cat >"$fake" <<'EOF'
#!/usr/bin/env bash
samples=1
counts=1
if [ "$AC013_SCALE_TREATMENT" = warm ]; then
  samples="$(awk 'BEGIN { for (i = 1; i <= 1000; i++) printf "%s1", (i == 1 ? "" : ",") }')"
  counts="$samples"
fi
printf 'AC013_TREATMENT_RECORD treatment=%s n=%s seed_write_ms=1 embedding_ms=not_separately_observable projection_drain_ms=1 accepted_writes=%s vector_rows_after_drain=%s drain_outcome=ok samples_us=%s result_counts=%s query_errors=0 query_timeouts=0 query_skips=0 query_invariant_failures=0\n' "$AC013_SCALE_TREATMENT" "$AC013_CORPUS_N" "$AC013_CORPUS_N" "$AC013_CORPUS_N" "$samples" "$counts" >"$LOG_PATH"
EOF
chmod +x "$fake"
out="$tmp/0.8.23-scale-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-raw"
AC013_V2_TEST_MODE=1 SCALE_OUTPUT_DIR="$out" AC013_RUNNER="$fake" bash "$root/scripts/perf-experiments/run-scale-ac013-matrix.sh"
test -f "$out/provenance.json"
test -f "$out/matrix-manifest.json"
python3 "$root/scripts/perf-experiments/ac013-v2.py" validate-root --test-fixture --root "$out"
find "$out" -maxdepth 1 -type f -printf '%f\n' >"$tmp/files"
file_count=0
while IFS= read -r _; do
  file_count=$((file_count + 1))
done <"$tmp/files"
test "$file_count" -eq 62
find "$out" -maxdepth 1 -type l | grep -q . && exit 1 || true
