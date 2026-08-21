#!/usr/bin/env bash
# scripts/release/smoke/smoke-npm-package.sh — AC-056 npm smoke.
#
#   $1 = version (e.g. 0.6.0)
#
# Installs fathomdb@$1 from npm into a fresh workspace, exercises the
# napi binding end-to-end (open + write minimal record + search + close),
# and asserts the node process exits cleanly. Same `feedback_release_
# verification` rationale as the PyPI smoke — locks + process exit are
# the bug signal that only fires under real install-from-registry.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  printf 'usage: %s <version>\n' "$0" >&2
  exit 2
fi
VERSION="$1"
if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'; then
  printf 'smoke-npm-package: invalid version "%s" — expected semver MAJOR.MINOR.PATCH[-PRERELEASE]\n' \
    "$VERSION" >&2
  exit 2
fi

# Resolved before the cd below: ${BASH_SOURCE[0]} is whatever relative or
# absolute path this script was invoked with, and dirname of a relative path
# only resolves correctly against the caller's original cwd.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cd "$WORK"
npm init -y >/dev/null
npm install --silent "fathomdb@${VERSION}"

# AC80-1/R80-2: the floor gate must bind the published bytes, not only a
# local build. Only Linux carries a glibc floor — `.node` is napi-rs's
# binary extension on every platform, so this is gated on the host OS, not
# merely on whether that file was found.
HOST_OS="$(uname -s)"
if [ "$HOST_OS" = "Linux" ]; then
  NATIVE_BINARY="$(find "$WORK/node_modules" -maxdepth 2 -type f -name 'fathomdb.*.node' -print -quit)"
  if [ -z "$NATIVE_BINARY" ]; then
    printf 'smoke-npm-package: no native .node binary found under node_modules on Linux — glibc-floor gate did not run\n' >&2
    exit 1
  fi
  # shellcheck source=/dev/null
  . "$REPO_ROOT/scripts/release/glibc-floor-contract.sh"
  case "$(basename "$NATIVE_BINARY")" in
    fathomdb.linux-x64-gnu.node) GLIBC_FAMILY='cuda-napi-host' ;;
    fathomdb.linux-arm64-gnu.node) GLIBC_FAMILY='manylinux' ;;
    *)
      printf 'smoke-npm-package: unsupported Linux native artifact for glibc-floor gate: %s\n' \
        "$NATIVE_BINARY" >&2
      exit 1
      ;;
  esac
  GLIBC_FLOOR="$(glibc_floor_for_family "$GLIBC_FAMILY")"
  bash "$REPO_ROOT/scripts/check-glibc-floor.sh" --floor "$GLIBC_FLOOR" "$NATIVE_BINARY"
fi

DB="$WORK/smoke.fdb"
cat > smoke.mjs <<'JS'
import { Engine } from "fathomdb";
const dbPath = process.argv[2];
const e = await Engine.open(dbPath);
await e.write([{ kind: "doc", body: "{}", sourceId: "smoke:npm-package" }]);
await e.search("smoke");
await e.close();
console.log("ok");
JS
node smoke.mjs "$DB"

printf 'smoke-npm-package: ok — fathomdb %s installed + open/write/search/close + process exit clean\n' \
  "$VERSION"
