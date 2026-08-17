#!/usr/bin/env bash
# V2 child-record failures must retain an explicit non-characterization result.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fake="$tmp/fake"
cat >"$fake" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
samples=1
counts=1
if [ "$AC013_SCALE_TREATMENT" = warm ]; then
  samples="$(awk 'BEGIN { for (i = 1; i <= 1000; i++) printf "%s1", (i == 1 ? "" : ",") }')"
  counts="$samples"
  if [ "${AC013_MALFORM_WARM:-}" = 1 ]; then counts=1; fi
fi
printf 'AC013_TREATMENT_RECORD treatment=%s n=%s seed_write_ms=1 embedding_ms=not_separately_observable projection_drain_ms=1 accepted_writes=%s vector_rows_after_drain=%s drain_outcome=ok samples_us=%s result_counts=%s query_errors=0 query_timeouts=0 query_skips=0 query_invariant_failures=0\n' \
  "$AC013_SCALE_TREATMENT" "$AC013_CORPUS_N" "$AC013_CORPUS_N" "$AC013_CORPUS_N" "$samples" "$counts" >"$LOG_PATH"
EOF
chmod +x "$fake"

failed="$tmp/0.8.23-scale-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-raw"
if AC013_V2_TEST_MODE=1 AC013_MALFORM_WARM=1 SCALE_OUTPUT_DIR="$failed" AC013_RUNNER="$fake" \
  bash "$root/scripts/perf-experiments/run-scale-ac013-matrix.sh"; then
  echo 'malformed warm record reached a complete root' >&2
  exit 1
fi
python3 - "$failed.status.json" "$failed/partial-manifest.json" <<'PY'
import hashlib, json, pathlib, sys
artifact = json.loads(pathlib.Path(sys.argv[1]).read_text())
partial = json.loads(pathlib.Path(sys.argv[2]).read_text())
assert artifact['status'] == 'ENVIRONMENT_INVALID', artifact
assert artifact['summary'] is None, artifact
assert artifact['matrix'] == [], artifact
final = partial['attempted_entries'][-1]
assert final['command_exit_status'] != 0, final
assert final['raw_sha256'] == hashlib.sha256((pathlib.Path(sys.argv[2]).parent / final['log']).read_bytes()).hexdigest(), final
PY
test ! -e "$failed/matrix-manifest.json"

partial="$tmp/0.8.23-scale-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-raw"
AC013_V2_TEST_MODE=1 AC013_V2_PARTIAL_RETAINED_AFTER=2 SCALE_OUTPUT_DIR="$partial" AC013_RUNNER="$fake" \
  bash "$root/scripts/perf-experiments/run-scale-ac013-matrix.sh"
python3 - "$partial.status.json" "$partial/partial-manifest.json" <<'PY'
import json, pathlib, sys
artifact = json.loads(pathlib.Path(sys.argv[1]).read_text())
manifest = json.loads(pathlib.Path(sys.argv[2]).read_text())
assert artifact['status'] == 'INSUFFICIENT_SAMPLES', artifact
assert artifact['summary'] is None, artifact
assert len(manifest['attempted_entries']) == 2, manifest
assert len(manifest['unrun_repetitions']) == 28, manifest
assert len(artifact['matrix']) == 1, artifact
assert len(artifact['matrix'][0]['repetitions']) == 2, artifact
PY
test ! -e "$partial/matrix-manifest.json"
