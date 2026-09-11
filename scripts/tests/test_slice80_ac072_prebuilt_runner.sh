#!/usr/bin/env bash
# Verify the 80.n AC-072 route invokes only its supplied test executable.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
runner="$root/scripts/perf-experiments/run-slice80-ac072-cell.sh"

test -x "$runner"
grep -F 'ac_013_vector_retrieval_latency' "$runner" >/dev/null
if rg -n 'cargo (test|build)|rustc|run-ac013.sh' "$runner"; then
  echo "prebuilt AC-072 route must not invoke a build wrapper" >&2
  exit 1
fi
grep -F 'run-slice80-ac072-cell' "$root/dev/tools/slice80_read_acceptance.py" >/dev/null

temp_dir=$(mktemp -d /tmp/fathomdb-slice80-prebuilt.XXXXXX)
cleanup() { rm -rf "$temp_dir"; }
trap cleanup EXIT
source_sha=$(git -C "$root" rev-parse HEAD)
input_sha=$(git -C "$root" ls-tree -r "$source_sha" -- \
  Cargo.toml Cargo.lock .cargo/config.toml \
  src/rust/crates/fathomdb-engine/Cargo.toml \
  src/rust/crates/fathomdb-engine/src \
  src/rust/crates/fathomdb-engine/tests/perf_gates.rs \
  src/rust/crates/fathomdb-engine/tests/reader_pool.rs \
  src/rust/crates/fathomdb-query src/rust/crates/fathomdb-schema \
  src/rust/crates/fathomdb-embedder src/rust/crates/fathomdb-embedder-api | sha256sum | awk '{print $1}')
capture="$temp_dir/invocation"
cat >"$temp_dir/fake-test" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$FAKE_CAPTURE/args"
printf '%s\n' "$AGENT_LONG,$AC013_CORPUS_N,$AC013_VECTOR_DIM,$AC013_SAMPLES,$AC013_SCALE_TREATMENT" >"$FAKE_CAPTURE/env"
printf 'test ac_013_vector_retrieval_latency ... ok\n'
printf 'test result: ok. 1 passed; 0 failed; 0 ignored\n'
EOF
chmod +x "$temp_dir/fake-test"
mkdir "$capture" "$temp_dir/bin"
for command in cargo rustc; do
  printf '#!/usr/bin/env bash\nexit 99\n' >"$temp_dir/bin/$command"
  chmod +x "$temp_dir/bin/$command"
done
binary_sha=$(sha256sum "$temp_dir/fake-test" | awk '{print $1}')
PATH="$temp_dir/bin:$PATH" FAKE_CAPTURE="$capture" "$runner" "$root" "$temp_dir/fake-test" \
  "$temp_dir/smoke.log" "$source_sha" "$binary_sha" "$input_sha" smoke 10
test "$(tr '\n' ' ' <"$capture/args")" = '--exact ac_013_vector_retrieval_latency --nocapture --test-threads=1 '
test "$(cat "$capture/env")" = '1,10,384,1000,warm'

echo "ok test_slice80_ac072_prebuilt_runner"
