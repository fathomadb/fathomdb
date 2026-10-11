#!/usr/bin/env bash
# Exercise the combined public-docs and retained Tegra package site.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILDER="${PAGES_SITE_BUILDER:-$REPO_ROOT/scripts/release/build-pages-site.sh}"
WORKFLOW="${DOCS_PAGES_CI_YML:-$REPO_ROOT/.github/workflows/docs-pages.yml}"
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
  pass "combined Pages site builder exists and is executable"
else
  fail "combined Pages site builder is absent or not executable: $BUILDER"
  printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
  exit 1
fi

if [ -f "$WORKFLOW" ]; then
  pass "public documentation Pages workflow exists"
else
  fail "public documentation Pages workflow is absent: $WORKFLOW"
fi

mkdir -p "$TMPROOT/bin"
cat > "$TMPROOT/bin/mkdocs" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
site_dir=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --site-dir) site_dir="${2:-}"; shift 2 ;;
    *) shift ;;
  esac
done
[ -n "$site_dir" ]
mkdir -p "$site_dir/reference"
printf '<!doctype html><title>FathomDB docs</title>\n' > "$site_dir/index.html"
printf '<!doctype html><title>Rust API</title>\n' > "$site_dir/reference/rust-api.html"
SH
chmod +x "$TMPROOT/bin/mkdocs"

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

# Retention is exercised in test_tegra_pages_retention.sh; here the retained
# list is empty so this test isolates the docs-plus-new-wheel layout.
EMPTY_MANIFEST="$TMPROOT/retained.json"
printf '{"schema": "fathomdb.tegra-pages-retained/v1", "artifacts": []}\n' > "$EMPTY_MANIFEST"
mkdir -p "$TMPROOT/retained"
OUT="$TMPROOT/site"
PATH="$TMPROOT/bin:$PATH" "$BUILDER" --manifest "$EMPTY_MANIFEST" --artifact-dir "$TMPROOT/retained" \
  --wheel "$WHEEL" --out "$OUT" --version "$VERSION"
WHEEL_NAME="$(basename "$WHEEL")"

if [ -f "$OUT/index.html" ] && [ -f "$OUT/reference/rust-api.html" ]; then
  pass "combined site retains the MkDocs root"
else
  fail "combined site lacks the MkDocs root"
fi

if [ -f "$OUT/tegra/simple/index.html" ] && [ -f "$OUT/tegra/packages/$WHEEL_NAME" ]; then
  pass "combined site retains the Tegra package index and wheel"
else
  fail "combined site lacks the Tegra package subtree"
fi

mkdir -p "$TMPROOT/nonempty"
printf 'keep\n' > "$TMPROOT/nonempty/existing.txt"
set +e
nonempty_out="$(PATH="$TMPROOT/bin:$PATH" "$BUILDER" \
  --manifest "$EMPTY_MANIFEST" --artifact-dir "$TMPROOT/retained" \
  --wheel "$WHEEL" --out "$TMPROOT/nonempty" --version "$VERSION" 2>&1)"
nonempty_rc=$?
set -e
if [ "$nonempty_rc" -ne 0 ] && grep -Fq 'nonempty Pages output directory' <<<"$nonempty_out"; then
  pass "combined builder refuses to replace a nonempty output"
else
  fail "combined builder did not fail closed for nonempty output: $nonempty_out"
fi

for contract in \
  'branches:' \
  '      - main' \
  'bash scripts/release/build-pages-site.sh' \
  'bash scripts/release/fetch-tegra-pages-retained.sh' \
  '--manifest scripts/release/tegra-pages-retained.json' \
  'scripts/release/tegra-pages-retained.json' \
  'actions/upload-pages-artifact@' \
  'actions/deploy-pages@' \
  'pages: write' \
  'id-token: write'; do
  if grep -Fq -- "$contract" "$WORKFLOW"; then
    pass "docs workflow retains contract: $contract"
  else
    fail "docs workflow lacks contract: $contract"
  fi
done

# The retained list is the one pin for Pages artifacts; an inline wheel pin in
# the docs deployer would let it diverge from the Jetson route.
for forbidden in 'TEGRA_WHEEL_SHA256' 'TEGRA_WHEEL_URL'; do
  if grep -Fq -- "$forbidden" "$WORKFLOW"; then
    fail "docs workflow pins a Tegra artifact outside the retained list: $forbidden"
  else
    pass "docs workflow has no inline Tegra artifact pin: $forbidden"
  fi
done

for forbidden in 'twine upload' 'npm publish' 'cargo publish'; do
  if grep -Fq -- "$forbidden" "$WORKFLOW"; then
    fail "docs workflow unexpectedly contains registry publication: $forbidden"
  else
    pass "docs workflow cannot publish through registry command: $forbidden"
  fi
done

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
