#!/usr/bin/env bash
# A Pages deployment is only correct if every retained Tegra artifact, and any
# artifact the run added, is served at its public https URL with its pinned
# SHA-256, and the generated index pages link each one. Both Pages deployers
# run verify-tegra-pages-live.sh after deploying
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decision 6).
#
# No network: a curl stand-in serves a fixture copy of the site, so each
# failure mode (404, digest mismatch, non-https URL, missing index link) is
# produced deliberately.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERIFY="$REPO_ROOT/scripts/release/verify-tegra-pages-live.sh"
DOCS_WORKFLOW="${DOCS_PAGES_CI_YML:-$REPO_ROOT/.github/workflows/docs-pages.yml}"
JETSON_WORKFLOW="${JETSON_TEGRA_CI_YML:-$REPO_ROOT/.github/workflows/jetson-tegra-cuda-evidence.yml}"
MANIFEST_REF='scripts/release/tegra-pages-retained.json'
SITE='https://fathomadb.github.io/fathomdb'
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

if [ -f "$VERIFY" ]; then
  pass 'live verification script exists'
else
  fail 'live verification script is absent: scripts/release/verify-tegra-pages-live.sh'
fi

# --- Both deployers verify after deploying ----------------------------------
job_block() {
  awk -v header="  $2:" '
    $0 == header { in_job = 1; next }
    in_job && /^  [A-Za-z0-9_-]+:$/ { exit }
    in_job { print }
  ' "$1"
}
assert_block() {
  local block="$1" needle="$2" description="$3"
  if [ -n "$block" ] && grep -Fq -- "$needle" <<<"$block"; then
    pass "$description"
  else
    fail "$description (missing: $needle)"
  fi
}
assert_block_absent() {
  local block="$1" needle="$2" description="$3"
  if [ -n "$block" ] && ! grep -Fq -- "$needle" <<<"$block"; then
    pass "$description"
  else
    fail "$description (unexpected or job absent: $needle)"
  fi
}
check_verifier_job() {
  local workflow="$1" job="$2" deploy_job="$3" label="$4" block
  block="$(job_block "$workflow" "$job")"
  assert_block "$block" "needs: $deploy_job" "$label verifies only after its deploy job"
  assert_block "$block" "bash scripts/release/verify-tegra-pages-live.sh --manifest $MANIFEST_REF" \
    "$label verifies every retained artifact live"
  assert_block "$block" $'    permissions:\n      contents: read' "$label verifier holds only read-only contents"
  assert_block_absent "$block" ': write' "$label verifier holds no write permission"
  assert_block_absent "$block" 'id-token' "$label verifier requests no OIDC token"
  assert_block "$block" 'persist-credentials: false' "$label verifier checkout keeps no credential"
}
check_verifier_job "$DOCS_WORKFLOW" verify-pages-live deploy-pages 'docs deployer'
check_verifier_job "$JETSON_WORKFLOW" verify-tegra-pages-live deploy-tegra-pages 'Jetson Pages route'
jetson_verify="$(job_block "$JETSON_WORKFLOW" verify-tegra-pages-live)"
assert_block "$jetson_verify" '--added "$WHEEL_FILENAME=$WHEEL_SHA256"' 'Jetson Pages route verifies the wheel it added'
assert_block "$jetson_verify" '--added "$NODE_TARBALL_FILENAME=$NODE_TARBALL_SHA256"' 'Jetson Pages route verifies the Node tarball it added'
assert_block "$jetson_verify" "if: \${{ inputs.publish_to_pages == true && startsWith(github.ref, 'refs/heads/release/') }}" \
  'Jetson Pages verifier runs exactly when publication runs'

# --- Behaviour against a fixture site ---------------------------------------
if [ ! -f "$VERIFY" ]; then
  printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
  exit 1
fi
FIXSITE="$TMPROOT/site"
mkdir -p "$FIXSITE/tegra/packages" "$FIXSITE/tegra/simple/fathomdb" "$FIXSITE/tegra/node" "$TMPROOT/bin"
WHEEL='fathomdb-0.8.26+tegra-cp310-abi3-linux_aarch64.whl'
NEW_WHEEL='fathomdb-0.8.27+tegra-cp310-abi3-linux_aarch64.whl'
NEW_TGZ='fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz'
printf 'retained wheel bytes\n' >"$FIXSITE/tegra/packages/$WHEEL"
printf 'new wheel bytes\n' >"$FIXSITE/tegra/packages/$NEW_WHEEL"
printf 'new tarball bytes\n' >"$FIXSITE/tegra/packages/$NEW_TGZ"
digest() { sha256sum "$1" | awk '{print $1}'; }
WHEEL_SHA="$(digest "$FIXSITE/tegra/packages/$WHEEL")"
NEW_WHEEL_SHA="$(digest "$FIXSITE/tegra/packages/$NEW_WHEEL")"
NEW_TGZ_SHA="$(digest "$FIXSITE/tegra/packages/$NEW_TGZ")"
write_indexes() {
  printf '<a href="../../packages/%s#sha256=%s">%s</a><br>\n<a href="../../packages/%s#sha256=%s">%s</a><br>\n' \
    "$WHEEL" "$WHEEL_SHA" "$WHEEL" "$NEW_WHEEL" "$NEW_WHEEL_SHA" "$NEW_WHEEL" \
    >"$FIXSITE/tegra/simple/fathomdb/index.html"
  printf '<a href="../packages/%s#sha256=%s">%s</a><br>\n' "$NEW_TGZ" "$NEW_TGZ_SHA" "$NEW_TGZ" \
    >"$FIXSITE/tegra/node/index.html"
}
write_indexes
write_manifest() {
  printf '{"schema":"fathomdb.tegra-pages-retained/v1","artifacts":[{"kind":"wheel","version":"0.8.26+tegra","filename":"%s","sha256":"%s","url":"%s"}]}\n' \
    "$WHEEL" "$1" "$2" >"$TMPROOT/manifest.json"
}
write_manifest "$WHEEL_SHA" "$SITE/tegra/packages/$WHEEL"

# curl stand-in: serves the fixture site for first-party https URLs, records
# every call, and answers 404 for anything absent.
cat >"$TMPROOT/bin/curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
url='' output='' write_out=''
printf '%s\n' "$*" >>"$CURL_LOG"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output) output="$2"; shift 2 ;;
    --write-out) write_out="$2"; shift 2 ;;
    --proto | --retry | --max-time | --connect-timeout) shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
prefix='https://fathomadb.github.io/fathomdb/'
case "$url" in
  "$prefix"*) path="${url#"$prefix"}" ;;
  *) printf 'curl stand-in refuses %s\n' "$url" >&2; exit 1 ;;
esac
case "$path" in */ | '') path="${path}index.html" ;; esac
if [ -f "$CURL_SITE/$path" ]; then
  cp "$CURL_SITE/$path" "$output"
  code=200
else
  : >"$output"
  code=404
fi
[ -z "$write_out" ] || printf '%s' "$code"
SH
chmod +x "$TMPROOT/bin/curl"

run_verify() {
  : >"$TMPROOT/curl.log"
  set +e
  CURL_SITE="$FIXSITE" CURL_LOG="$TMPROOT/curl.log" PATH="$TMPROOT/bin:$PATH" \
    bash "$VERIFY" --manifest "$TMPROOT/manifest.json" --max-wait-seconds 0 "$@" >"$TMPROOT/verify.out" 2>&1
  local rc=$?
  set -e
  return "$rc"
}
expect_pass() {
  local label="$1"
  shift
  if run_verify "$@"; then pass "$label"; else fail "$label"; cat "$TMPROOT/verify.out" >&2; fi
}
expect_fail() {
  local label="$1" needle="$2"
  shift 2
  if run_verify "$@"; then
    fail "$label (verification passed)"
  elif grep -Fq -- "$needle" "$TMPROOT/verify.out"; then
    pass "$label"
  else
    fail "$label (diagnostic lacks: $needle)"
    cat "$TMPROOT/verify.out" >&2
  fi
}

expect_pass 'a correct deployment of the retained list verifies'
unrestricted="$(grep -v -- "--proto =https" "$TMPROOT/curl.log" || true)"
if grep -q -- "--proto =https" "$TMPROOT/curl.log" && [ -z "$unrestricted" ]; then
  pass 'every download is restricted to https'
else
  fail 'every curl call must pass --proto =https'
  cat "$TMPROOT/curl.log" >&2
fi
if grep -Fq "$SITE/tegra/simple/fathomdb/" "$TMPROOT/curl.log"; then
  pass 'the PEP 503 project page is fetched live'
else
  fail 'the PEP 503 project page must be fetched live'
fi
expect_pass 'a correct deployment with the added wheel and tarball verifies' \
  --added "$NEW_WHEEL=$NEW_WHEEL_SHA" --added "$NEW_TGZ=$NEW_TGZ_SHA"
if grep -Fq "$SITE/tegra/node/" "$TMPROOT/curl.log"; then
  pass 'the Node tarball page is fetched live when a tarball is expected'
else
  fail 'the Node tarball page must be fetched live when a tarball is expected'
fi

expect_fail 'an added artifact that is not served (404) fails' 'HTTP 404' \
  --added "fathomdb-linux-arm64-gnu-tegra-0.8.28.tgz=$NEW_TGZ_SHA"
write_manifest "$NEW_TGZ_SHA" "$SITE/tegra/packages/$WHEEL"
expect_fail 'a retained artifact whose served bytes differ fails' 'SHA-256'
write_manifest "$WHEEL_SHA" "http://fathomadb.github.io/fathomdb/tegra/packages/$WHEEL"
expect_fail 'a non-https retained URL fails' 'https'
if ! grep -q 'http://' "$TMPROOT/curl.log"; then
  pass 'a non-https URL is never requested'
else
  fail 'a non-https URL must never be requested'
fi
write_manifest "$WHEEL_SHA" "$SITE/tegra/packages/$WHEEL"
expect_fail 'an added artifact with a malformed digest is a usage error' 'SHA-256' \
  --added "$NEW_TGZ=not-a-digest"
expect_fail 'an added path-bearing filename is refused' 'filename' \
  --added "../$NEW_TGZ=$NEW_TGZ_SHA"
printf '<a href="../../packages/%s#sha256=%s">%s</a>\n' "$NEW_WHEEL" "$NEW_WHEEL_SHA" "$NEW_WHEEL" \
  >"$FIXSITE/tegra/simple/fathomdb/index.html"
expect_fail 'a PEP 503 index missing a retained wheel link fails' 'index'
write_indexes
printf '<html></html>\n' >"$FIXSITE/tegra/node/index.html"
expect_fail 'a Node index missing the added tarball link fails' 'index' \
  --added "$NEW_TGZ=$NEW_TGZ_SHA"
write_indexes
printf '<a href="../../packages/%s#sha256=%s">%s</a>\n' "$WHEEL" "$NEW_TGZ_SHA" "$WHEEL" \
  >"$FIXSITE/tegra/simple/fathomdb/index.html"
expect_fail 'an index link with the wrong digest fails' 'index'
write_indexes
expect_fail 'an unbounded wait is refused' 'max-wait-seconds' --max-wait-seconds 100000

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
