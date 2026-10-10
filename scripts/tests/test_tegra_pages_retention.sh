#!/usr/bin/env bash
# Every Pages deployment replaces the whole site, so a published Tegra wheel
# or Node tarball survives only if each deployer rebuilds it. One
# digest-pinned retained-artifact list feeds both deployers
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decision 6). This test
# proves the list is well formed, that both deployers consume it through the
# same fetch and site-builder scripts, and that those scripts keep every
# listed artifact byte-for-byte while adding a new wheel and tarball.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MANIFEST="${TEGRA_PAGES_RETAINED:-$REPO_ROOT/scripts/release/tegra-pages-retained.json}"
HELPER="$REPO_ROOT/scripts/release/tegra-pages-retained.py"
FETCH="$REPO_ROOT/scripts/release/fetch-tegra-pages-retained.sh"
BUILDER="$REPO_ROOT/scripts/release/build-pages-site.sh"
DOCS_WORKFLOW="${DOCS_PAGES_CI_YML:-$REPO_ROOT/.github/workflows/docs-pages.yml}"
JETSON_WORKFLOW="${JETSON_TEGRA_CI_YML:-$REPO_ROOT/.github/workflows/jetson-tegra-cuda-evidence.yml}"
PAGES_BASE='https://fathomadb.github.io/fathomdb/tegra/packages'
MANIFEST_REF='scripts/release/tegra-pages-retained.json'
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

for file in "$MANIFEST" "$HELPER" "$FETCH" "$BUILDER"; do
  if [ -f "$file" ]; then
    pass "retention component exists: ${file#"$REPO_ROOT"/}"
  else
    fail "retention component is absent: ${file#"$REPO_ROOT"/}"
  fi
done
if [ "$FAILED" -ne 0 ]; then
  printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
  exit 1
fi

# --- The checked-in list ------------------------------------------------------
if python3 "$HELPER" validate --manifest "$MANIFEST"; then
  pass "checked-in retained-artifact list validates"
else
  fail "checked-in retained-artifact list must validate"
fi
if python3 - "$MANIFEST" <<'PY'
import json
import sys

manifest = json.load(open(sys.argv[1]))
wheels = {(a["filename"], a["sha256"]) for a in manifest["artifacts"] if a["kind"] == "wheel"}
assert (
    "fathomdb-0.8.26+tegra-cp310-abi3-linux_aarch64.whl",
    "728df8628f45e12c0c04c0a971c174764ef5c988c7d05f0737794aa6f5c5d1fa",
) in wheels, wheels
PY
then
  pass "the published 0.8.26+tegra wheel stays listed with its digest"
else
  fail "the published 0.8.26+tegra wheel must stay listed with its digest"
fi

expect_invalid() {
  local label="$1" needle="$2" body="$3" out
  printf '%s\n' "$body" > "$TMPROOT/invalid.json"
  set +e
  out="$(python3 "$HELPER" validate --manifest "$TMPROOT/invalid.json" 2>&1)"
  local rc=$?
  set -e
  if [ "$rc" -ne 0 ] && grep -Fq -- "$needle" <<<"$out"; then
    pass "$label"
  else
    fail "$label (rc=$rc): $out"
  fi
}
W='fathomdb-0.8.26+tegra-cp310-abi3-linux_aarch64.whl'
T='fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz'
D='728df8628f45e12c0c04c0a971c174764ef5c988c7d05f0737794aa6f5c5d1fa'
entry() { printf '{"kind":"%s","version":"%s","filename":"%s","sha256":"%s","url":"%s"}' "$@"; }
wheel_entry="$(entry wheel 0.8.26+tegra "$W" "$D" "$PAGES_BASE/$W")"
expect_invalid 'wrong schema is rejected' 'schema' \
  "{\"schema\":\"other\",\"artifacts\":[$wheel_entry]}"
expect_invalid 'a malformed digest is rejected' 'sha256' \
  "{\"schema\":\"fathomdb.tegra-pages-retained/v1\",\"artifacts\":[$(entry wheel 0.8.26+tegra "$W" abc "$PAGES_BASE/$W")]}"
expect_invalid 'a duplicate filename is rejected' 'duplicate' \
  "{\"schema\":\"fathomdb.tegra-pages-retained/v1\",\"artifacts\":[$wheel_entry,$wheel_entry]}"
expect_invalid 'a URL other than its first-party Pages location is rejected' 'url' \
  "{\"schema\":\"fathomdb.tegra-pages-retained/v1\",\"artifacts\":[$(entry wheel 0.8.26+tegra "$W" "$D" "https://example.invalid/$W")]}"
expect_invalid 'an unknown artifact kind is rejected' 'kind' \
  "{\"schema\":\"fathomdb.tegra-pages-retained/v1\",\"artifacts\":[$(entry sdist 0.8.26+tegra "$W" "$D" "$PAGES_BASE/$W")]}"
expect_invalid 'a wheel filename must carry its +tegra version' 'filename' \
  "{\"schema\":\"fathomdb.tegra-pages-retained/v1\",\"artifacts\":[$(entry wheel 0.8.27+tegra "$W" "$D" "$PAGES_BASE/$W")]}"
expect_invalid 'a Node tarball filename must name the Tegra package and version' 'filename' \
  "{\"schema\":\"fathomdb.tegra-pages-retained/v1\",\"artifacts\":[$(entry node-tarball 0.8.28 "$T" "$D" "$PAGES_BASE/$T")]}"
expect_invalid 'a path-bearing filename is rejected' 'filename' \
  "{\"schema\":\"fathomdb.tegra-pages-retained/v1\",\"artifacts\":[$(entry node-tarball 0.8.27 "../$T" "$D" "$PAGES_BASE/../$T")]}"

# --- Both deployers consume the one list through the same scripts ------------
job_block() {
  local workflow="$1" job_name="$2"
  awk -v job_name="$job_name" '
    $0 == "  " job_name ":" { in_job = 1; next }
    in_job && /^  [A-Za-z0-9_-]+:$/ { exit }
    in_job { print }
  ' "$workflow"
}
assert_job_contains() {
  local workflow="$1" job_name="$2" needle="$3" description="$4" block
  block="$(job_block "$workflow" "$job_name")"
  if grep -Fq -- "$needle" <<<"$block"; then
    pass "$description"
  else
    fail "$description (missing from $job_name: $needle)"
  fi
}
check_deployers() {
  local docs="$1" jetson="$2"
  assert_job_contains "$docs" prepare-pages "bash scripts/release/fetch-tegra-pages-retained.sh --manifest $MANIFEST_REF" \
    "docs deployer fetches every listed artifact"
  assert_job_contains "$docs" prepare-pages "--manifest $MANIFEST_REF --artifact-dir" \
    "docs deployer rebuilds the site from the retained list"
  assert_job_contains "$jetson" prepare-tegra-pages "bash scripts/release/fetch-tegra-pages-retained.sh --manifest $MANIFEST_REF" \
    "Jetson Pages route fetches every listed artifact"
  assert_job_contains "$jetson" prepare-tegra-pages "--manifest $MANIFEST_REF --artifact-dir" \
    "Jetson Pages route rebuilds the site from the retained list"
  assert_job_contains "$jetson" prepare-tegra-pages '--node-tarball "$tarball"' \
    "Jetson Pages route publishes the Node tarball with the wheel"
  assert_job_contains "$jetson" prepare-tegra-pages '--wheel "$wheel"' \
    "Jetson Pages route publishes the wheel with the Node tarball"
}
check_deployers "$DOCS_WORKFLOW" "$JETSON_WORKFLOW"

# A deployer that stops passing the list must fail the check above.
sed "s|--manifest $MANIFEST_REF --artifact-dir|--artifact-dir|" "$DOCS_WORKFLOW" > "$TMPROOT/docs-mutated.yml"
before="$FAILED"
check_deployers "$TMPROOT/docs-mutated.yml" "$JETSON_WORKFLOW" > /dev/null 2>&1 || true
if [ "$FAILED" -gt "$before" ]; then
  FAILED="$before"
  pass "mutation proves the docs deployer cannot drop the retained list"
else
  fail "dropping the retained list from the docs deployer went unnoticed"
fi
sed "s|--manifest $MANIFEST_REF --artifact-dir|--artifact-dir|" "$JETSON_WORKFLOW" > "$TMPROOT/jetson-mutated.yml"
before="$FAILED"
check_deployers "$DOCS_WORKFLOW" "$TMPROOT/jetson-mutated.yml" > /dev/null 2>&1 || true
if [ "$FAILED" -gt "$before" ]; then
  FAILED="$before"
  pass "mutation proves the Jetson Pages route cannot drop the retained list"
else
  fail "dropping the retained list from the Jetson Pages route went unnoticed"
fi

# --- Fetch and build keep every listed artifact ---------------------------------
FIXTURES="$TMPROOT/published"
mkdir -p "$FIXTURES" "$TMPROOT/bin"
make_wheel() {
  python3 - "$1" "$2" <<'PY'
import sys
import zipfile

wheel, version = sys.argv[1:]
with zipfile.ZipFile(wheel, "w") as archive:
    archive.writestr(
        f"fathomdb-{version}.dist-info/METADATA",
        f"Metadata-Version: 2.1\nName: fathomdb\nVersion: {version}\n",
    )
PY
}
make_tarball() {
  python3 - "$1" "$2" "${3:-}" <<'PY'
import io
import json
import sys
import tarfile

path, version, variant = sys.argv[1:]
members = {
    "package/package.json": json.dumps({"name": "fathomdb-linux-arm64-gnu-tegra", "version": version}).encode(),
    "package/fathomdb.linux-arm64-gnu.node": b"\x7fELF fixture " + version.encode() + variant.encode(),
}
with tarfile.open(path, "w:gz") as archive:
    for name, data in members.items():
        info = tarfile.TarInfo(name)
        info.size = len(data)
        archive.addfile(info, io.BytesIO(data))
PY
}
OLD_WHEEL="fathomdb-0.8.24+tegra-cp310-abi3-linux_aarch64.whl"
MID_WHEEL="fathomdb-0.8.26+tegra-cp310-abi3-linux_aarch64.whl"
OLD_TGZ="fathomdb-linux-arm64-gnu-tegra-0.8.26.tgz"
make_wheel "$FIXTURES/$OLD_WHEEL" 0.8.24+tegra
make_wheel "$FIXTURES/$MID_WHEEL" 0.8.26+tegra
make_tarball "$FIXTURES/$OLD_TGZ" 0.8.26
digest() { sha256sum "$1" | awk '{print $1}'; }
python3 - "$TMPROOT/fixture-manifest.json" "$FIXTURES" "$PAGES_BASE" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

out, fixtures, base = sys.argv[1:]
artifacts = []
for name, kind, version in (
    ("fathomdb-0.8.24+tegra-cp310-abi3-linux_aarch64.whl", "wheel", "0.8.24+tegra"),
    ("fathomdb-0.8.26+tegra-cp310-abi3-linux_aarch64.whl", "wheel", "0.8.26+tegra"),
    ("fathomdb-linux-arm64-gnu-tegra-0.8.26.tgz", "node-tarball", "0.8.26"),
):
    data = (Path(fixtures) / name).read_bytes()
    artifacts.append({
        "kind": kind,
        "version": version,
        "filename": name,
        "sha256": hashlib.sha256(data).hexdigest(),
        "url": f"{base}/{name}",
    })
Path(out).write_text(json.dumps({"schema": "fathomdb.tegra-pages-retained/v1", "artifacts": artifacts}, indent=2) + "\n")
PY
FIXTURE_MANIFEST="$TMPROOT/fixture-manifest.json"

# curl stand-in: serves the published fixtures for first-party Pages URLs only.
cat > "$TMPROOT/bin/curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
url='' output=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output) output="$2"; shift 2 ;;
    --proto | --retry) shift 2 ;;
    https://*) url="$1"; shift ;;
    *) shift ;;
  esac
done
prefix='https://fathomadb.github.io/fathomdb/tegra/packages/'
case "$url" in
  "$prefix"*) cp "$CURL_FIXTURES/${url#"$prefix"}" "$output" ;;
  *) printf 'curl stand-in refuses %s\n' "$url" >&2; exit 22 ;;
esac
SH
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
mkdir -p "$site_dir"
printf '<!doctype html><title>FathomDB docs</title>\n' > "$site_dir/index.html"
SH
chmod +x "$TMPROOT/bin/curl" "$TMPROOT/bin/mkdocs"
stub_path="$TMPROOT/bin:$PATH"

RETAINED="$TMPROOT/retained"
if CURL_FIXTURES="$FIXTURES" PATH="$stub_path" bash "$FETCH" --manifest "$FIXTURE_MANIFEST" --dest "$RETAINED" \
  && cmp -s "$FIXTURES/$OLD_WHEEL" "$RETAINED/$OLD_WHEEL" \
  && cmp -s "$FIXTURES/$MID_WHEEL" "$RETAINED/$MID_WHEEL" \
  && cmp -s "$FIXTURES/$OLD_TGZ" "$RETAINED/$OLD_TGZ"; then
  pass "fetch retrieves every listed artifact byte-for-byte"
else
  fail "fetch must retrieve every listed artifact byte-for-byte"
fi

cp "$FIXTURES/$MID_WHEEL" "$TMPROOT/tampered.whl"
mkdir -p "$TMPROOT/tampered-published"
cp "$FIXTURES"/* "$TMPROOT/tampered-published/"
printf 'tamper' >> "$TMPROOT/tampered-published/$MID_WHEEL"
set +e
tamper_out="$(CURL_FIXTURES="$TMPROOT/tampered-published" PATH="$stub_path" bash "$FETCH" \
  --manifest "$FIXTURE_MANIFEST" --dest "$TMPROOT/tampered-retained" 2>&1)"
tamper_rc=$?
set -e
if [ "$tamper_rc" -ne 0 ] && grep -Fq 'SHA-256' <<<"$tamper_out"; then
  pass "fetch fails closed when a published artifact no longer matches its digest"
else
  fail "fetch accepted bytes that differ from the retained digest: $tamper_out"
fi

NEW_WHEEL="$TMPROOT/new/fathomdb-0.8.27+tegra-cp310-abi3-linux_aarch64.whl"
NEW_TGZ="$TMPROOT/new/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz"
mkdir -p "$TMPROOT/new"
make_wheel "$NEW_WHEEL" 0.8.27+tegra
make_tarball "$NEW_TGZ" 0.8.27

SITE="$TMPROOT/site"
set +e
site_out="$(PATH="$stub_path" bash "$BUILDER" --manifest "$FIXTURE_MANIFEST" --artifact-dir "$RETAINED" \
  --wheel "$NEW_WHEEL" --version 0.8.27+tegra --node-tarball "$NEW_TGZ" --node-version 0.8.27 \
  --out "$SITE" 2>&1)"
site_rc=$?
set -e
if [ "$site_rc" -eq 0 ]; then
  pass "site builder accepts the retained list plus a new wheel and tarball"
else
  fail "site builder failed: $site_out"
fi
missing=''
for name in "$OLD_WHEEL" "$MID_WHEEL" "$OLD_TGZ"; do
  cmp -s "$FIXTURES/$name" "$SITE/tegra/packages/$name" || missing="$missing $name"
done
if [ -z "$missing" ]; then
  pass "site keeps every listed artifact byte-for-byte"
else
  fail "site dropped or altered listed artifact(s):$missing"
fi
if cmp -s "$NEW_WHEEL" "$SITE/tegra/packages/$(basename "$NEW_WHEEL")" \
  && cmp -s "$NEW_TGZ" "$SITE/tegra/packages/$(basename "$NEW_TGZ")"; then
  pass "site publishes the new wheel and Node tarball together"
else
  fail "site must publish the new wheel and Node tarball together"
fi
index_ok=1
for wheel in "$FIXTURES/$OLD_WHEEL" "$FIXTURES/$MID_WHEEL" "$NEW_WHEEL"; do
  needle="../../packages/$(basename "$wheel")#sha256=$(digest "$wheel")"
  grep -Fq -- "$needle" "$SITE/tegra/simple/fathomdb/index.html" || index_ok=0
done
if [ "$index_ok" -eq 1 ]; then
  pass "PEP 503 project page links every wheel with its SHA-256"
else
  fail "PEP 503 project page must link every wheel with its SHA-256"
fi
node_ok=1
for tarball in "$FIXTURES/$OLD_TGZ" "$NEW_TGZ"; do
  needle="../packages/$(basename "$tarball")#sha256=$(digest "$tarball")"
  grep -Fq -- "$needle" "$SITE/tegra/node/index.html" || node_ok=0
done
if [ "$node_ok" -eq 1 ]; then
  pass "Node tarball page links every tarball with its SHA-256"
else
  fail "Node tarball page must link every tarball with its SHA-256"
fi
if grep -Fxq "node_tarball_sha256=$(digest "$NEW_TGZ")" <<<"$site_out" \
  && grep -Fxq "wheel_sha256=$(digest "$NEW_WHEEL")" <<<"$site_out"; then
  pass "site builder reports the new wheel and tarball digests"
else
  fail "site builder must report wheel_sha256= and node_tarball_sha256=: $site_out"
fi

expect_builder_failure() {
  local label="$1" needle="$2" out rc
  shift 2
  set +e
  out="$(PATH="$stub_path" bash "$BUILDER" "$@" 2>&1)"
  rc=$?
  set -e
  if [ "$rc" -ne 0 ] && grep -Fq -- "$needle" <<<"$out"; then
    pass "$label"
  else
    fail "$label (rc=$rc): $out"
  fi
}
mkdir -p "$TMPROOT/partial"
cp "$RETAINED/$OLD_WHEEL" "$RETAINED/$OLD_TGZ" "$TMPROOT/partial/"
expect_builder_failure 'a listed artifact absent from the artifact directory fails the build' \
  "$MID_WHEEL" --manifest "$FIXTURE_MANIFEST" --artifact-dir "$TMPROOT/partial" --out "$TMPROOT/site-partial"
mkdir -p "$TMPROOT/altered"
cp "$RETAINED"/* "$TMPROOT/altered/"
printf 'x' >> "$TMPROOT/altered/$OLD_TGZ"
expect_builder_failure 'a listed artifact whose bytes changed fails the build' \
  'SHA-256' --manifest "$FIXTURE_MANIFEST" --artifact-dir "$TMPROOT/altered" --out "$TMPROOT/site-altered"
CONFLICT_TGZ="$TMPROOT/conflict/$OLD_TGZ"
mkdir -p "$TMPROOT/conflict"
make_tarball "$CONFLICT_TGZ" 0.8.26 rebuilt
expect_builder_failure 'a new artifact cannot replace a retained one under the same name' \
  'retained' --manifest "$FIXTURE_MANIFEST" --artifact-dir "$RETAINED" \
  --node-tarball "$CONFLICT_TGZ" --node-version 0.8.26 --out "$TMPROOT/site-conflict"
WRONG_TGZ="$TMPROOT/wrong/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz"
mkdir -p "$TMPROOT/wrong"
make_tarball "$WRONG_TGZ" 0.8.25
expect_builder_failure 'a Node tarball whose package version differs is refused' \
  'version' --manifest "$FIXTURE_MANIFEST" --artifact-dir "$RETAINED" \
  --node-tarball "$WRONG_TGZ" --node-version 0.8.27 --out "$TMPROOT/site-wrong"
expect_builder_failure 'the site builder requires the retained list' \
  'usage' --artifact-dir "$RETAINED" --wheel "$NEW_WHEEL" --version 0.8.27+tegra --out "$TMPROOT/site-nolist"

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
