#!/usr/bin/env bash
# scripts/release/smoke/smoke-pypi-wheel.sh — AC-056 PyPI smoke.
#
#   $1 = version (e.g. 0.6.0)
#
# Installs fathomdb==$1 from PyPI into a fresh venv, exercises the SDK
# with exact persisted reads and a missing lookup in separate processes. Per `feedback_release_verification`:
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

python3 -m venv "$WORK/venv"
# shellcheck source=/dev/null
. "$WORK/venv/bin/activate"
pip install --quiet --upgrade pip

# Bounded retry on transient PyPI CDN/simple-index skew only; non-skew errors
# (including synthetic non-zero fixture exits) fail immediately with exact status.
max_install_attempts="${FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS:-5}"
if ! [[ "$max_install_attempts" =~ ^[1-9][0-9]*$ ]]; then
  printf 'smoke-pypi-wheel: FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS must be >= 1 (got "%s")\n' "$max_install_attempts" >&2
  exit 2
fi
retry_delay_secs="${FATHOMDB_SMOKE_PYPI_RETRY_DELAY_SECS:-15}"
attempt=1
while true; do
  install_exit=0
  pip install --quiet "fathomdb[cli]==${PIP_VERSION}" > "$WORK/pip-install.log" 2>&1 || install_exit=$?
  if [ "$install_exit" -eq 0 ]; then
    break
  fi
  if [ "$attempt" -lt "$max_install_attempts" ] \
    && grep -qE "No matching distribution found for (fathomdb|fathomdb-cli)==|Could not find a version that satisfies the requirement (fathomdb|fathomdb-cli)==" "$WORK/pip-install.log"; then
    printf 'smoke-pypi-wheel: transient PyPI simple-index skew on attempt %s/%s; retrying in %ss\n' \
      "$attempt" "$max_install_attempts" "$retry_delay_secs" >&2
    attempt=$((attempt + 1))
    if [ "$retry_delay_secs" -gt 0 ]; then
      sleep "$retry_delay_secs"
    fi
    continue
  fi
  cat "$WORK/pip-install.log" >&2
  exit "$install_exit"
done

CLI_BIN="$WORK/venv/bin/fathomdb"
if [ ! -x "$CLI_BIN" ]; then
  printf 'smoke-pypi-wheel: expected CLI binary %s not installed by fathomdb[cli]==%s\n' \
    "$CLI_BIN" "$PIP_VERSION" >&2
  exit 1
fi
if [ -e "$WORK/venv/bin/perf-regression-check" ]; then
  printf 'smoke-pypi-wheel: unexpected perf-regression-check binary shipped in fathomdb-cli wheel\n' >&2
  exit 1
fi

# AC80-1/R80-2: the floor gate must bind the published bytes, not only a
# local build. Only Linux carries a glibc floor — macOS's extension module
# is also named _fathomdb.abi3.so, so this is gated on the host OS, not
# merely on whether that file was found.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
HOST_OS="$(uname -s)"
set --
if [ "$HOST_OS" = "Linux" ]; then
  NATIVE_BINARY="$(find "$WORK/venv" -type f -name '_fathomdb.abi3.so' -print -quit)"
  if [ -z "$NATIVE_BINARY" ]; then
    printf 'smoke-pypi-wheel: no native _fathomdb.abi3.so found under the venv on Linux — glibc-floor gate did not run\n' >&2
    exit 1
  fi
  # shellcheck source=/dev/null
  . "$REPO_ROOT/scripts/release/glibc-floor-contract.sh"
  bash "$REPO_ROOT/scripts/check-glibc-floor.sh" --floor "$GLIBC_FLOOR" "$NATIVE_BINARY"
  bash "$REPO_ROOT/scripts/check-glibc-floor.sh" --floor "$GLIBC_FLOOR" "$CLI_BIN"
  set -- "$NATIVE_BINARY"
fi

# Verify fathomdb + fathomdb-cli co-install cleanly (`_coinstall.py` lock) and
# that the packaged CLI binary executes without a Rust toolchain on PATH.
"${WORK}/venv/bin/python" -c "import fathomdb; print(fathomdb.__version__)"
"$CLI_BIN" --version
"$CLI_BIN" doctor platform --json

DB="$WORK/smoke.fdb"
consumer="$REPO_ROOT/scripts/release/smoke/registry-python.py"
"${WORK}/venv/bin/python" -I "$consumer" "$PIP_VERSION" "$DB" write "$@"
"${WORK}/venv/bin/python" -I "$consumer" "$PIP_VERSION" "$DB" reopen "$@"

env -u PYTHONPATH python3 "$REPO_ROOT/src/python/tests/smoke_slice55_installed.py"

smoke_complete=1
printf 'smoke-pypi-wheel: ok — fathomdb[cli] %s installed + CLI + Slice 55 native trace/explain/refusals + process exit clean\n' \
  "$VERSION"
