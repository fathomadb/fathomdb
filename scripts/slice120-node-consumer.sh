#!/usr/bin/env bash
# Compile and run the Slice 120 package-root fixtures from an isolated install.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
native_binary=${FATHOMDB_SLICE120_NATIVE:?set FATHOMDB_SLICE120_NATIVE to the candidate linux-x64-gnu .node artifact}
os_name=$(uname -s)
architecture=$(uname -m)
if [[ $os_name != Linux || $architecture != x86_64 ]]; then
  echo "Slice 120 installed consumer requires linux-x64-gnu" >&2
  exit 2
fi
if [[ ! -f $native_binary ]]; then
  echo "native artifact missing: $native_binary" >&2
  exit 2
fi
node_version=$(node --version)
if [[ $node_version != v25.9.0 ]]; then
  echo "Slice 120 installed consumer requires Node v25.9.0" >&2
  exit 2
fi
source_status=$(git -C "$repo_root" status --porcelain -- \
  src/ts/src src/ts/tests/fixtures scripts/slice120-node-consumer.sh)
if [[ -n $source_status ]]; then
  echo "Slice 120 installed consumer requires committed SDK source and fixtures" >&2
  exit 1
fi
candidate_sha=$(git -C "$repo_root" rev-parse HEAD)

run_dir=$(mktemp -d /tmp/fathomdb-slice120-consumer-XXXXXX)
trap 'rm -rf -- "$run_dir"' EXIT
export npm_config_cache="$run_dir/npm-cache"
mkdir -p "$run_dir/platform" "$run_dir/packs" "$run_dir/consumer"
cp "$repo_root/src/ts/npm/linux-x64-gnu/package.json" \
  "$repo_root/src/ts/npm/linux-x64-gnu/LICENSE" "$run_dir/platform/"
cp "$native_binary" "$run_dir/platform/fathomdb.linux-x64-gnu.node"

(
  cd "$repo_root/src/ts"
  node node_modules/typescript/bin/tsc -p tsconfig.build.json
  npm pack --ignore-scripts --pack-destination "$run_dir/packs" --silent
)
(
  cd "$run_dir/platform"
  npm pack --ignore-scripts --pack-destination "$run_dir/packs" --silent
)
version=$(node -p "require('$repo_root/src/ts/package.json').version")
npm install --offline --ignore-scripts --no-audit --no-fund --package-lock=false \
  --prefix "$run_dir/consumer" \
  "$run_dir/packs/fathomdb-$version.tgz" \
  "$run_dir/packs/fathomdb-linux-x64-gnu-$version.tgz"
cp "$repo_root/src/ts/tests/fixtures/consumer-package-root.mts" "$run_dir/consumer/consumer.mts"
cp "$repo_root/src/ts/tests/fixtures/consumer-package-root.mjs" "$run_dir/consumer/consumer.mjs"
node "$repo_root/src/ts/node_modules/typescript/bin/tsc" --ignoreConfig --noEmit \
  --strict --module NodeNext --moduleResolution NodeNext --target ES2022 \
  --skipLibCheck "$run_dir/consumer/consumer.mts"
(
  cd "$run_dir/consumer"
  node consumer.mjs
)
sha256sum "$run_dir/packs/fathomdb-$version.tgz" \
  "$run_dir/packs/fathomdb-linux-x64-gnu-$version.tgz" "$native_binary"
npm_version=$(npm --version)
printf 'Slice 120 installed consumer PASS: source=%s node=%s npm=%s\n' \
  "$candidate_sha" "$node_version" "$npm_version"
