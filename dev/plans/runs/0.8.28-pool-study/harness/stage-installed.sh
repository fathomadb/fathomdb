#!/usr/bin/env bash
# 0.8.28 pool study: installed-package staging (protocol section 3), adapted
# from fix-verification/stage-installed.sh without file copies. Packs the thin
# main package from the built tree (src/ts with dist/) and a linux-arm64-gnu
# platform package from <platform-dir> (its package.json plus the .node that
# `napi build ... <platform-dir>` wrote there), then installs both offline with
# lifecycle scripts disabled into a fresh consumer directory. The main
# package's optionalDependencies are not injected; Node resolves the platform
# package from the consumer's node_modules all the same.
# Usage: stage-installed.sh <repo-root> <platform-dir> <staging-dir> <consumer-dir>
set -euo pipefail
repo=$1; platform=$2; staging=$3; consumer=$4
# shellcheck source=/dev/null
source ~/.nvm/nvm.sh >/dev/null && nvm use 25.9.0 >/dev/null
for d in "$staging" "$consumer"; do
  if [ -e "$d" ]; then echo "refusing to reuse existing $d" >&2; exit 1; fi
done
mkdir -p "$staging" "$consumer"
platform_tgz="$staging/$(cd "$platform" && npm pack --silent --pack-destination "$staging")"
main_tgz="$staging/$(cd "$repo/src/ts" && npm pack --silent --pack-destination "$staging")"
cd "$consumer"
npm init -y >/dev/null
npm install --offline --ignore-scripts --no-audit --no-fund "$platform_tgz" "$main_tgz"
sha256sum "$platform/fathomdb.linux-arm64-gnu.node" \
  "$consumer/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node" "$platform_tgz" "$main_tgz"
tar -tzf "$platform_tgz"
tar -tzf "$main_tgz" | head -20
