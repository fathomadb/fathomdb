#!/usr/bin/env bash
# scripts/release/smoke/smoke-pypi-mcp.sh — AC-056 PyPI fathomdb[mcp] wheel smoke.
#
#   $1 = version (e.g. 0.8.32)
#
# Installs fathomdb[mcp]==$1 from PyPI into a fresh venv, verifies co-installed
# fathomdb SDK import, asserts the Linux glibc floor on the packaged
# fathomdb-mcp binary, and runs the stdio MCP initialize + tools/list smoke.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  printf 'usage: %s <version>\n' "$0" >&2
  exit 2
fi
VERSION="$1"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  printf 'smoke-pypi-mcp: invalid version "%s" — expected semver MAJOR.MINOR.PATCH[-PRERELEASE]\n' \
    "$VERSION" >&2
  exit 2
fi

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

max_install_attempts="${FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS:-5}"
if ! [[ "$max_install_attempts" =~ ^[1-9][0-9]*$ ]]; then
  printf 'smoke-pypi-mcp: FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS must be >= 1 (got "%s")\n' "$max_install_attempts" >&2
  exit 2
fi
retry_delay_secs="${FATHOMDB_SMOKE_PYPI_RETRY_DELAY_SECS:-15}"
attempt=1
while true; do
  install_exit=0
  pip install --quiet "fathomdb[mcp]==${PIP_VERSION}" > "$WORK/pip-install.log" 2>&1 || install_exit=$?
  if [ "$install_exit" -eq 0 ]; then
    break
  fi
  if [ "$attempt" -lt "$max_install_attempts" ] \
    && grep -qE "No matching distribution found for (fathomdb|fathomdb-mcp)==|Could not find a version that satisfies the requirement (fathomdb|fathomdb-mcp)==" "$WORK/pip-install.log"; then
    printf 'smoke-pypi-mcp: transient PyPI simple-index skew on attempt %s/%s; retrying in %ss\n' \
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

MCP_BIN="$WORK/venv/bin/fathomdb-mcp"
if [ ! -x "$MCP_BIN" ]; then
  printf 'smoke-pypi-mcp: expected MCP binary %s not installed by fathomdb[mcp]==%s\n' \
    "$MCP_BIN" "$PIP_VERSION" >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
HOST_OS="$(uname -s)"
if [ "$HOST_OS" = "Linux" ]; then
  # shellcheck source=/dev/null
  . "$REPO_ROOT/scripts/release/glibc-floor-contract.sh"
  bash "$REPO_ROOT/scripts/check-glibc-floor.sh" --floor "$GLIBC_FLOOR" "$MCP_BIN"
fi

"${WORK}/venv/bin/python" -c "import fathomdb; print(fathomdb.__version__)"
python3 "$REPO_ROOT/scripts/release/smoke/mcp-stdio-smoke.py" --binary "$MCP_BIN"

smoke_complete=1
printf 'smoke-pypi-mcp: ok — fathomdb[mcp] %s installed and passed stdio smoke\n' \
  "$VERSION"
