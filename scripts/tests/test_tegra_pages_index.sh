#!/usr/bin/env bash
# Slice 30: exercise the static PEP 503 tree with a real wheel-shaped fixture.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILDER="${TEGRA_PAGES_INDEX_BUILDER:-$REPO_ROOT/scripts/release/build-tegra-pages-index.sh}"
PASSED=0
FAILED=0

pass() { printf 'PASS  %s\n' "$1"; PASSED=$((PASSED + 1)); }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

TMPROOT="$(mktemp -d)"
cleanup() {
  case "$TMPROOT" in
    "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$TMPROOT" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$TMPROOT" >&2 ;;
  esac
}
trap cleanup EXIT

if [ -x "$BUILDER" ]; then
  pass "Tegra Pages index builder exists and is executable"
else
  fail "Tegra Pages index builder is absent or not executable: $BUILDER"
  printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
  exit 1
fi

VERSION='0.8.24+tegra'
WHEEL="$TMPROOT/fathomdb-${VERSION}-cp310-abi3-linux_aarch64.whl"
python3 - "$WHEEL" "$VERSION" <<'PY'
import sys
import zipfile

wheel, version = sys.argv[1:]
with zipfile.ZipFile(wheel, 'w') as archive:
    archive.writestr(
        f'fathomdb-{version}.dist-info/METADATA',
        f'Metadata-Version: 2.1\nName: fathomdb\nVersion: {version}\n',
    )
PY

OUT="$TMPROOT/site"
"$BUILDER" --wheel "$WHEEL" --out "$OUT" --version "$VERSION"
WHEEL_NAME="$(basename "$WHEEL")"
WHEEL_SHA256="$(sha256sum "$WHEEL" | awk '{print $1}')"

if grep -Fq '<a href="fathomdb/">fathomdb</a>' "$OUT/tegra/simple/index.html"; then
  pass "PEP 503 root names the normalized fathomdb project"
else
  fail "PEP 503 root lacks the normalized fathomdb project"
fi

if grep -Fq "../../packages/${WHEEL_NAME}#sha256=${WHEEL_SHA256}" "$OUT/tegra/simple/fathomdb/index.html"; then
  pass "project page links the exact wheel with its SHA-256"
else
  fail "project page lacks the exact SHA-256 wheel link"
fi

if cmp -s "$WHEEL" "$OUT/tegra/packages/$WHEEL_NAME"; then
  pass "static site retains the verified wheel bytes"
else
  fail "static site wheel bytes differ from the verified input"
fi

mkdir -p "$TMPROOT/bad"
BAD_WHEEL="$TMPROOT/bad/$WHEEL_NAME"
python3 - "$BAD_WHEEL" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1], 'w') as archive:
    archive.writestr(
        'fathomdb-0.8.24+tegra.dist-info/METADATA',
        'Metadata-Version: 2.1\nName: fathomdb\nVersion: 0.8.24\n',
    )
PY
set +e
bad_out="$("$BUILDER" --wheel "$BAD_WHEEL" --out "$TMPROOT/bad-site" --version "$VERSION" 2>&1)"
bad_rc=$?
set -e
if [ "$bad_rc" -ne 0 ] && grep -Fq "Version: ${VERSION}" <<<"$bad_out"; then
  pass "mismatched wheel metadata fails closed"
else
  fail "mismatched wheel metadata did not fail closed: $bad_out"
fi

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
