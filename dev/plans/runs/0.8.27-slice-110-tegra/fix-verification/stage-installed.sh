#!/usr/bin/env bash
# Slice 110 allocator-fix verification: pack the thin main package and the
# linux-arm64-gnu platform package from a built tree, then install both
# offline, with lifecycle scripts disabled, into a fresh consumer directory
# outside the source tree (the receipt's installed-package form).
# Usage: stage-installed.sh <repo-root> <staging-dir> <consumer-dir>
set -euo pipefail
repo=$1; staging=$2; consumer=$3
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
source ~/.nvm/nvm.sh >/dev/null && nvm use 25.9.0 >/dev/null
for d in "$staging" "$consumer"; do
  if [ -e "$d" ]; then echo "refusing to reuse existing $d" >&2; exit 1; fi
done
main="$staging/main"; platforms="$staging/platforms"; platform="$platforms/linux-arm64-gnu"
mkdir -p "$main" "$platform" "$consumer"
cp "$repo/src/ts/package.json" "$repo/src/ts/LICENSE" "$main/"
cp -R "$repo/src/ts/dist" "$main/dist"
cp "$repo/src/ts/npm/linux-arm64-gnu/package.json" "$repo/src/ts/npm/linux-arm64-gnu/LICENSE" \
  "$repo/src/ts/npm/linux-arm64-gnu/README.md" "$platform/"
cp "$repo/src/ts/fathomdb.linux-arm64-gnu.node" "$platform/fathomdb.linux-arm64-gnu.node"
bash "$repo/scripts/release/npm-inject-optional-deps.sh" "$main" "$platforms"
platform_tgz="$platform/$(cd "$platform" && npm pack --silent)"
main_tgz="$main/$(cd "$main" && npm pack --silent)"
cd "$consumer"
npm init -y >/dev/null
npm install --offline --ignore-scripts --no-audit --no-fund "$platform_tgz" "$main_tgz"
cp "$here/consumer.mjs" "$consumer/consumer.mjs"
installed="$consumer/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node"
sha256sum "$repo/src/ts/fathomdb.linux-arm64-gnu.node" "$installed" "$platform_tgz" "$main_tgz"
tar -tzf "$platform_tgz"
tar -tzf "$main_tgz"
