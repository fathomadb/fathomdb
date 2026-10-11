#!/usr/bin/env bash
# Build and qualify Node artifacts from one exact, clean Slice 90 source commit.
set -euo pipefail

script_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

check_source() {
  local source_root=$1
  local sha
  local source_status
  sha=$(git -C "$source_root" rev-parse HEAD)
  source_status=$(git -C "$source_root" status --porcelain --untracked-files=all)
  if [[ -n $source_status ]]; then
    echo "qualification source is dirty or uncommitted: $source_root" >&2
    return 1
  fi
  printf '%s\n' "$sha"
}

if [[ ${1:-} == --check-source ]]; then
  check_source "${2:-$script_root}"
  exit
fi
if [[ $# -ne 0 ]]; then
  echo "usage: $0 [--check-source [git-repository]]" >&2
  exit 2
fi
os_name=$(uname -s)
architecture=$(uname -m)
if [[ $os_name != Linux || $architecture != x86_64 ]]; then
  echo "this installed qualification packages linux-x64-gnu only" >&2
  exit 2
fi

candidate_sha=$(check_source "$script_root")
node_binary=${FATHOMDB_QUALIFICATION_NODE_BIN:-$(command -v node)}
node_directory=$(dirname "$node_binary")
export PATH="$node_directory:$PATH"
qualification_dir=$(mktemp -d "/tmp/fathomdb-node-${candidate_sha:0:10}-XXXXXX")
export npm_config_cache="$qualification_dir/npm-cache"
mkdir -p "$qualification_dir/artifacts/main" "$qualification_dir/artifacts/production" \
  "$qualification_dir/artifacts/witness" "$qualification_dir/staging/production" \
  "$qualification_dir/staging/witness"

export CARGO_TARGET_DIR="$qualification_dir/cargo-target"
(
  cd "$script_root/src/rust"
  cargo build --release -p fathomdb-napi --features default-embedder 2>&1 |
    tee "$qualification_dir/production-build.log"
)
cp "$CARGO_TARGET_DIR/release/libfathomdb_napi.so" "$qualification_dir/production.node"

(
  cd "$script_root/src/ts"
  npm exec -- tsc -p tsconfig.build.json
  npm pack --pack-destination "$qualification_dir/artifacts/main" --silent
)

stage_native() {
  local variant=$1
  local binary=$2
  local stage="$qualification_dir/staging/$variant"
  cp "$script_root/src/ts/npm/linux-x64-gnu/package.json" "$stage/"
  cp "$script_root/src/ts/npm/linux-x64-gnu/LICENSE" "$stage/"
  cp "$script_root/src/ts/npm/linux-x64-gnu/README.md" "$stage/"
  cp "$binary" "$stage/fathomdb.linux-x64-gnu.node"
  (cd "$stage" && npm pack --pack-destination "$qualification_dir/artifacts/$variant" --silent)
}

stage_native production "$qualification_dir/production.node"

(
  cd "$script_root/src/rust"
  cargo build --release -p fathomdb-napi --features default-embedder,test-hooks 2>&1 |
    tee "$qualification_dir/witness-build.log"
)
cp "$CARGO_TARGET_DIR/release/libfathomdb_napi.so" "$qualification_dir/witness.node"
stage_native witness "$qualification_dir/witness.node"

run_consumer() {
  local variant=$1
  local consumer="$qualification_dir/consumer-$variant"
  mkdir -p "$consumer"
  npm install --offline --ignore-scripts --prefix "$consumer" \
    "$qualification_dir/artifacts/main/fathomdb-0.8.26.tgz" \
    "$qualification_dir/artifacts/$variant/fathomdb-linux-x64-gnu-0.8.26.tgz" 2>&1 |
    tee "$qualification_dir/$variant-install.log"
  cp "$script_root/src/ts/tests/slice90-installed-qualification.mjs" "$consumer/qualify.mjs"
  (
    cd "$consumer"
    FATHOMDB_QUALIFICATION_MODE="$variant" "$node_binary" qualify.mjs 2>&1 |
      tee "$qualification_dir/$variant-test.log"
  )
}

run_consumer production
run_consumer witness
final_sha=$(check_source "$script_root")
if [[ $candidate_sha != "$final_sha" ]]; then
  echo "source HEAD changed during installed qualification" >&2
  exit 1
fi
node_version=$("$node_binary" --version)
npm_version=$(npm --version)
rustc_version=$(rustc --version)

{
  printf 'candidate_sha=%s\n' "$candidate_sha"
  printf 'source_clean_before=true\nsource_clean_after=true\n'
  printf 'os=%s\narch=%s\n' "$os_name" "$architecture"
  printf 'node=%s\nnpm=%s\nrustc=%s\n' \
    "$node_version" "$npm_version" "$rustc_version"
  printf 'production_features=default-embedder\n'
  printf 'witness_features=default-embedder,test-hooks\n'
  printf 'production_command=cargo build --release -p fathomdb-napi --features default-embedder\n'
  printf 'witness_command=cargo build --release -p fathomdb-napi --features default-embedder,test-hooks\n'
  printf 'runner_command=FATHOMDB_QUALIFICATION_NODE_BIN=%s bash scripts/slice90-node-installed-qualification.sh\n' "$node_binary"
  sha256sum "$qualification_dir/production.node" "$qualification_dir/witness.node" \
    "$qualification_dir/artifacts/main/fathomdb-0.8.26.tgz" \
    "$qualification_dir/artifacts/production/fathomdb-linux-x64-gnu-0.8.26.tgz" \
    "$qualification_dir/artifacts/witness/fathomdb-linux-x64-gnu-0.8.26.tgz"
  printf 'production_test=PASS\nwitness_test=PASS\n'
} > "$qualification_dir/receipt.txt"

cat > "$qualification_dir/evidence-map.md" <<EOF
# Slice 90 Node installed evidence map

Candidate: \`$candidate_sha\`. See \`receipt.txt\` for exact artifact hashes,
build features, host versions, and commands. Both installed consumers resolve
the SDK and native package from their own \`node_modules\`; neither links to
the workspace.

| Setting | Installed native forwarding | Installed consuming effect | Rust owner effect |
| --- | --- | --- | --- |
| \`scheduler_runtime_threads\` | Test-feature native getter reads the Rust engine's requested value at 1, 4, and 64. | Native connection inventory proves exact 1, 4, and 64 projection workers. | Engine owner tests cover admission and cleanup; this receipt does not replace them. |
| \`embedder_pool_size\` | Test-feature native getter reads distinct 3 and ceiling 64. | No provider is attached by public Node open, so no embed worker or provider concurrency effect is claimed. | Deterministic caller-provider capacity tests belong to the Rust engine owner. |
| \`embedder_call_timeout_ms\` | Test-feature native getter reads 12,345 ms; installed minimum and maximum open. | No caller provider is exposed by public Node open, so no provider timeout effect is claimed. | Deadline and timeout outcome tests belong to the Rust engine owner. |
| \`provenance_row_cap\` | Test-feature native getter reads the JavaScript safe maximum; installed minimum and maximum open. | Production installed SQLite rows prove cap 1 prunes and cap 0 retains. | Rust owner tests cover retention semantics and exemptions. |
| \`slow_threshold_ms\` | Test-feature native getter reads 456 ms; installed minimum and safe maximum open. | The requested snapshot stays frozen after the setter; the current Node subscriber accepts calls without delivering events, so no slow-event signal effect is claimed. | Operation and SQLite profile signal tests belong to the Rust engine owner with a lifecycle subscriber. |

The test-feature getter is absent from the production artifact. Native open
validation, requested-value checks, and the explicit TypeScript config object
handoff together establish five-field forwarding. Provider, timeout, and
slow-event consuming effects require the engine-owner receipts listed above.
EOF

printf 'qualification receipt: %s\n' "$qualification_dir/receipt.txt"
