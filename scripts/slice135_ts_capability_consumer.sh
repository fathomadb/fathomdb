#!/usr/bin/env bash
# Pack exact candidate SDK/native bytes, install in an external consumer, and retain raw exercise output.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
result_root=${1:?usage: script RESULT_DIR NATIVE_NODE SOURCE_SHA}
native_binary=${2:?native .node path required}
source_sha=${3:?40-character source SHA required}
git -C "$repo_root" cat-file -e "$source_sha^{commit}"
git -C "$repo_root" diff --quiet "$source_sha" -- \
  src/rust src/ts/src src/ts/package.json src/ts/npm/linux-x64-gnu/package.json Cargo.lock
node_root=/home/coreyt/.nvm/versions/node/v25.9.0
export PATH="$node_root/bin:$PATH"
test "$(node --version)" = v25.9.0
test "$(uname -s)" = Linux
test "$(uname -m)" = x86_64
test -f "$native_binary"
test ! -e "$result_root"
mkdir -p "$result_root/main" "$result_root/platform" "$result_root/packs" "$result_root/consumer" "$result_root/compiled"

cp "$repo_root/src/ts/package.json" "$repo_root/src/ts/LICENSE" "$result_root/main/"
cp "$repo_root/src/ts/npm/linux-x64-gnu/package.json" "$repo_root/src/ts/npm/linux-x64-gnu/LICENSE" "$result_root/platform/"
cp "$native_binary" "$result_root/platform/fathomdb.linux-x64-gnu.node"
node "$repo_root/src/ts/node_modules/typescript/bin/tsc" -p "$repo_root/src/ts/tsconfig.build.json" --outDir "$result_root/main/dist"
node "$repo_root/src/ts/node_modules/typescript/bin/tsc" -p "$repo_root/src/ts/tsconfig.json" --outDir "$result_root/compiled"
(
  cd "$result_root/main"
  npm pack --ignore-scripts --pack-destination "$result_root/packs" --silent
)
(
  cd "$result_root/platform"
  npm pack --ignore-scripts --pack-destination "$result_root/packs" --silent
)
version=$(node -p "require('$repo_root/src/ts/package.json').version")
npm install --offline --ignore-scripts --no-audit --no-fund --package-lock=false \
  --prefix "$result_root/consumer" \
  "$result_root/packs/fathomdb-$version.tgz" \
  "$result_root/packs/fathomdb-linux-x64-gnu-$version.tgz" \
  > "$result_root/install.stdout" 2> "$result_root/install.stderr"
native_sha=$(sha256sum "$native_binary" | cut -d ' ' -f 1)
printf '%s\n' "$source_sha" > "$result_root/product-source-sha.txt"
node "$repo_root/scripts/slice135_ts_capabilities.mjs" \
  --repo "$repo_root" \
  --compiled-root "$result_root/compiled" \
  --install-root "$result_root/consumer" \
  --canonical "$repo_root/src/conformance/governed-operation-parity.json" \
  --source-sha "$source_sha" \
  --native-sha256 "$native_sha" \
  --main-archive "$result_root/packs/fathomdb-$version.tgz" \
  --native-archive "$result_root/packs/fathomdb-linux-x64-gnu-$version.tgz" \
  --output "$result_root/raw.json" \
  > "$result_root/runner.stdout" 2> "$result_root/runner.stderr"
sha256sum "$result_root/packs/"*.tgz "$result_root/raw.json" \
  "$result_root/platform/fathomdb.linux-x64-gnu.node" > "$result_root/SHA256SUMS"
