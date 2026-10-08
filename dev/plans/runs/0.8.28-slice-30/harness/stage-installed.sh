#!/usr/bin/env bash
# 0.8.28 Slice 30 qualification: installed-package staging for the Node addon.
# Packs the thin main package from the built tree (src/ts with dist/) and a
# linux-arm64-gnu platform package from <platform-dir> (its package.json plus
# the .node that `napi build ... <platform-dir>` wrote there), then installs
# both offline with lifecycle scripts disabled into a fresh consumer
# directory. Node resolves the platform package from the consumer's
# node_modules.
# Usage: stage-installed.sh <repo-root> <platform-dir> <staging-dir> <consumer-dir>
set -euo pipefail
repo=$1
platform=$2
staging=$3
consumer=$4
# shellcheck source=/dev/null
source ~/.nvm/nvm.sh >/dev/null && nvm use "${NODE_VERSION:-25.9.0}" >/dev/null
for d in "$staging" "$consumer"; do
  if [ -e "$d" ]; then
    echo "refusing to reuse existing $d" >&2
    exit 1
  fi
done
mkdir -p "$staging" "$consumer"
platform_name=$(cd "$platform" && npm pack --silent --pack-destination "$staging")
main_name=$(cd "$repo/src/ts" && npm pack --silent --pack-destination "$staging")
platform_tgz="$staging/$platform_name"
main_tgz="$staging/$main_name"
cd "$consumer"
npm init -y >/dev/null
npm install --offline --ignore-scripts --no-audit --no-fund "$platform_tgz" "$main_tgz"
sha256sum "$platform/fathomdb.linux-arm64-gnu.node" \
  "$consumer/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node" "$platform_tgz" "$main_tgz"
