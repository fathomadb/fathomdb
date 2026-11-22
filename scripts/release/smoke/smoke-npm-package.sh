#!/usr/bin/env bash
# scripts/release/smoke/smoke-npm-package.sh — AC-056 npm smoke.
#
#   $1 = version (e.g. 0.6.0)
#
# Installs fathomdb@$1 from npm into a fresh workspace, exercises the
# matching napi binding, native trace controls and persisted record reads,
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
smoke_complete=0
cleanup() {
  local status=$? cleanup_status=0
  trap - EXIT
  rm -rf "$WORK" || cleanup_status=$?
  # Bash 3 can enter EXIT with status 0 after a fatal nounset expansion.
  if [ "$status" -eq 0 ]; then
    if [ "$smoke_complete" -ne 1 ]; then status=1; else status=$cleanup_status; fi
  fi
  exit "$status"
}
trap cleanup EXIT

cd "$WORK"
npm init -y >/dev/null
install_exit=0
npm install --loglevel=warn "fathomdb@${VERSION}" > "$WORK/npm-install.log" 2>&1 || install_exit=$?
consumer="$REPO_ROOT/scripts/release/smoke/registry-node.mjs"
check_exit=0
NATIVE_BINARY="$(node "$consumer" inspect "$WORK" "$VERSION" "$WORK/npm-install.log" "$install_exit")" || check_exit=$?
if [ "$install_exit" -ne 0 ]; then exit "$install_exit"; fi
if [ "$check_exit" -ne 0 ]; then exit "$check_exit"; fi

# AC80-1/R80-2: the floor gate must bind the published bytes, not only a
# local build. Only Linux carries a glibc floor — `.node` is napi-rs's
# binary extension on every platform, so this is gated on the host OS, not
# merely on whether that file was found.
HOST_OS="$(uname -s)"
if [ "$HOST_OS" = "Linux" ]; then
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

node "$consumer" write "$WORK" "$VERSION"
node "$consumer" reopen "$WORK" "$VERSION"

smoke_complete=1
printf 'smoke-npm-package: ok — fathomdb %s installed + native trace controls + independent write and reopen processes exited cleanly\n' "$VERSION"
