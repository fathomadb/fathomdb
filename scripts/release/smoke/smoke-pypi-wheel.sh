#!/usr/bin/env bash
# scripts/release/smoke/smoke-pypi-wheel.sh — AC-056 PyPI smoke.
#
#   $1 = version (e.g. 0.6.0)
#
# Installs fathomdb==$1 from PyPI into a fresh venv, exercises the SDK
# end-to-end (open + write minimal record + search + close), and asserts
# the python process exits cleanly. Per `feedback_release_verification`:
# the wheel-on-disk lock cleanup + process exit are the bug signal that
# only fires under real install-from-registry conditions.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  printf 'usage: %s <version>\n' "$0" >&2
  exit 2
fi
VERSION="$1"
if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'; then
  printf 'smoke-pypi-wheel: invalid version "%s" — expected semver MAJOR.MINOR.PATCH[-PRERELEASE]\n' \
    "$VERSION" >&2
  exit 2
fi

# PEP 440 normalization: PyPI stores pre-releases without the hyphen/dot
# separators that SemVer uses (0.6.0-rc.2 -> 0.6.0rc2). maturin emits PEP
# 440 versions when building the wheel, so the registry-side version
# differs from the workflow's $GITHUB_REF_NAME-derived tag version.
pep440_normalize() {
  local v="$1"
  case "$v" in
    *-rc.*)    printf '%s' "${v%-rc.*}rc${v##*-rc.}" ;;
    *-alpha.*) printf '%s' "${v%-alpha.*}a${v##*-alpha.}" ;;
    *-beta.*)  printf '%s' "${v%-beta.*}b${v##*-beta.}" ;;
    *)         printf '%s' "$v" ;;
  esac
}
PIP_VERSION="$(pep440_normalize "$VERSION")"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

python3 -m venv "$WORK/venv"
# shellcheck source=/dev/null
. "$WORK/venv/bin/activate"
pip install --quiet --upgrade pip
pip install --quiet "fathomdb==${PIP_VERSION}"

# AC80-1/R80-2: the floor gate must bind the published bytes, not only a
# local build. Only Linux carries a glibc floor — macOS's extension module
# is also named _fathomdb.abi3.so, so this is gated on the host OS, not
# merely on whether that file was found.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
HOST_OS="$(uname -s)"
if [ "$HOST_OS" = "Linux" ]; then
  NATIVE_BINARY="$(find "$WORK/venv" -type f -name '_fathomdb.abi3.so' -print -quit)"
  if [ -z "$NATIVE_BINARY" ]; then
    printf 'smoke-pypi-wheel: no native _fathomdb.abi3.so found under the venv on Linux — glibc-floor gate did not run\n' >&2
    exit 1
  fi
  # shellcheck source=/dev/null
  . "$REPO_ROOT/scripts/release/glibc-floor-contract.sh"
  bash "$REPO_ROOT/scripts/check-glibc-floor.sh" --floor "$GLIBC_FLOOR" "$NATIVE_BINARY"
fi

DB="$WORK/smoke.fdb"
python3 - "$DB" <<'PY'
import sys
from fathomdb import Engine
db_path = sys.argv[1]
e = Engine.open(db_path)
e.write([{"kind": "doc", "body": "{}", "source_id": "smoke:pypi-wheel"}])
e.search("smoke")
e.close()
print("ok")
PY

printf 'smoke-pypi-wheel: ok — fathomdb %s installed + open/write/search/close + process exit clean\n' \
  "$VERSION"
