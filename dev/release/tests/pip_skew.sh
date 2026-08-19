#!/usr/bin/env bash
# AC-051b: pip version-skew detected at resolve time.
#
# Builds four synthetic wheels (api-v1, api-v2, probe-a, probe-b) where
# probe-a pins mock-skew-api==0.6.0 and probe-b pins
# mock-skew-api==99.99.99. Installing both into one environment forces
# the pip resolver to fail with a conflict naming mock-skew-api. The
# stand-in names map back to the real packages once REQ-048 publishing
# lands (see fixtures/pip-skew/constraints.txt).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="$SCRIPT_DIR/../fixtures/pip-skew"
# shellcheck source=../../../scripts/lib/agent-python-env.sh
. "$SCRIPT_DIR/../../../scripts/lib/agent-python-env.sh"

if ! command -v python3 >/dev/null 2>&1; then
  echo "skip: python3 not on PATH" >&2
  exit 0
fi

if ! python3 -c 'import setuptools, wheel' >/dev/null 2>&1; then
  echo "skip: setuptools/wheel not importable; install python build deps" >&2
  exit 0
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

VENV="$WORK/venv"
WHEELS="$WORK/wheels"
SRC="$WORK/src"
mkdir -p "$WHEELS"

# Copy the fixture into a writable workdir so the wheel build does not
# leak build/ or *.egg-info under the tracked fixture path.
cp -r "$FIXTURE" "$SRC"

# The fixture's build tools were already verified in the host interpreter.
# Inherit them so this resolver-only test stays offline: upgrading them here
# makes a fresh venv contact PyPI before the local `--no-index` assertion.
python3 -m venv --system-site-packages "$VENV"
PIP="$VENV/bin/pip"

# Build wheels for the two api versions + two probes into a local
# find-links directory. --no-build-isolation avoids a network fetch of
# the build backend; the inherited setuptools+wheel were verified above.
for pkg in api-v1 api-v2 probe-a probe-b; do
  "$PIP" wheel --quiet --no-deps --no-build-isolation \
    --wheel-dir "$WHEELS" "$SRC/$pkg" >/dev/null
done

# 0.8.23 Slice 80.3: `--dry-run` needs pip >=22.2. The venv above deliberately
# keeps the OLD system-bundled pip (its ensurepip-vendored version, e.g. 22.0.2
# on Ubuntu 22.04/every Jetson) to inherit system-site-packages setuptools/wheel
# for the offline wheel BUILD step above — upgrading pip in that venv would
# need network access, breaking this test's whole offline invariant. The
# RESOLVE step below needs no setuptools/wheel (it installs already-built
# wheel files, it builds nothing), so it can use a different, modern
# interpreter's pip instead — no new venv needed, `--dry-run` never installs.
RESOLVE_PYTHON="$(select_python_for_venv)" || {
  echo "skip: no Python 3.11+ available to run the modern pip --dry-run check (system pip is too old for --dry-run)" >&2
  exit 0
}
if ! "$RESOLVE_PYTHON" -m pip --version >/dev/null 2>&1; then
  echo "skip: $RESOLVE_PYTHON has no importable pip module (ensurepip never ran there)" >&2
  exit 0
fi

# Resolve both probes against the local index only. Expect failure.
if out="$("$RESOLVE_PYTHON" -m pip install --dry-run --no-index \
  --find-links "$WHEELS" \
  mock-fathomdb==0.6.0 mock-fathomdb-embedder==0.6.0 2>&1)"; then
  printf 'FAIL: pip resolved unexpectedly; expected conflict\n%s\n' "$out" >&2
  exit 1
fi

if ! printf '%s' "$out" | grep -q 'mock-skew-api'; then
  printf 'FAIL: pip error did not name mock-skew-api\n%s\n' "$out" >&2
  exit 1
fi

printf 'PASS: AC-051b — pip resolver detected mock-skew-api skew\n'
