#!/usr/bin/env bash
# Every Pages deployment replaces the whole site, so a deployment built from a
# retained list that lags the live site silently withdraws whatever the list
# misses. Before deploying, both Pages deployers compare the live Tegra index
# pages with the retained list plus the run's own additions, and refuse to
# deploy if anything live would vanish, unless the run names that artifact as
# a deliberate withdrawal (ADR-0.8.27-jetson-tegra-node-addon-distribution,
# decision 6).
#
# No network: a curl stand-in serves a fixture copy of the live site, so each
# case (unretained live artifact, digest conflict, absent index, transient
# server error) is produced deliberately.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECK="$REPO_ROOT/scripts/release/check-tegra-pages-retention-live.sh"
DOCS_WORKFLOW="${DOCS_PAGES_CI_YML:-$REPO_ROOT/.github/workflows/docs-pages.yml}"
JETSON_WORKFLOW="${JETSON_TEGRA_CI_YML:-$REPO_ROOT/.github/workflows/jetson-tegra-cuda-evidence.yml}"
CHECK_CALL='bash scripts/release/check-tegra-pages-retention-live.sh --manifest scripts/release/tegra-pages-retained.json'
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

if [ -f "$CHECK" ]; then
  pass 'live retention check script exists'
else
  fail 'live retention check script is absent: scripts/release/check-tegra-pages-retention-live.sh'
fi

# --- Both deployers check retention before deploying ------------------------
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
# The check must run in the job that builds the site, before the artifact the
# deploy job publishes is uploaded.
assert_before() {
  local block="$1" first="$2" second="$3" description="$4" first_line second_line
  first_line="$(grep -m1 -nF -- "$first" <<<"$block" | cut -d: -f1 || true)"
  second_line="$(grep -m1 -nF -- "$second" <<<"$block" | cut -d: -f1 || true)"
  if [ -n "$first_line" ] && [ -n "$second_line" ] && [ "$first_line" -lt "$second_line" ]; then
    pass "$description"
  else
    fail "$description ($first must precede $second)"
  fi
}
check_deployer() {
  local workflow="$1" prepare_job="$2" deploy_job="$3" label="$4" prepare deploy dispatch interpolations
  prepare="$(job_block "$workflow" "$prepare_job")"
  deploy="$(job_block "$workflow" "$deploy_job")"
  assert_block "$prepare" "$CHECK_CALL" "$label checks live retention while preparing the site"
  assert_before "$prepare" "$CHECK_CALL" 'actions/upload-pages-artifact@' \
    "$label checks live retention before uploading the Pages artifact"
  assert_block "$deploy" "needs: $prepare_job" "$label deploys only after the retention check job"
  assert_block "$prepare" '--withdraw "$WITHDRAW_TEGRA_ARTIFACTS"' \
    "$label passes the deliberate withdrawal input to the check"
  assert_block "$prepare" 'WITHDRAW_TEGRA_ARTIFACTS: ${{ inputs.withdraw_tegra_artifacts }}' \
    "$label passes the withdrawal input through the environment"
  dispatch="$(awk '/^  workflow_dispatch:/ { on = 1; next } on && /^[^ ]/ { exit } on && /^  [a-z_]+:/ { exit } on { print }' "$workflow")"
  assert_block "$dispatch" '      withdraw_tegra_artifacts:' "$label declares the withdraw_tegra_artifacts dispatch input"
  interpolations="$(grep -cF '${{ inputs.withdraw_tegra_artifacts }}' "$workflow" || true)"
  if [ "$interpolations" = 1 ]; then
    pass "$label never interpolates the withdrawal input into a script"
  else
    fail "$label must reference the withdrawal input exactly once, in env (found $interpolations)"
  fi
}
check_deployer "$DOCS_WORKFLOW" prepare-pages deploy-pages 'docs deployer'
check_deployer "$JETSON_WORKFLOW" prepare-tegra-pages deploy-tegra-pages 'Jetson Pages route'
docs_paths="$(awk '/^  push:/ { on = 1; next } on && /^  [a-z_]+:/ { exit } on { print }' "$DOCS_WORKFLOW")"
assert_block "$docs_paths" '- scripts/release/check-tegra-pages-retention-live.sh' \
  'a change to the retention check redeploys the docs site'
jetson_prepare="$(job_block "$JETSON_WORKFLOW" prepare-tegra-pages)"
assert_block "$jetson_prepare" '--added "$WHEEL_FILENAME=$WHEEL_SHA256"' \
  'Jetson Pages route counts the wheel it adds as retained'
assert_block "$jetson_prepare" '--added "$NODE_TARBALL_FILENAME=$NODE_TARBALL_SHA256"' \
  'Jetson Pages route counts the Node tarball it adds as retained'

# --- Behaviour against a fixture live site ----------------------------------
if [ ! -f "$CHECK" ]; then
  printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
  exit 1
fi
FIXSITE="$TMPROOT/site"
mkdir -p "$FIXSITE" "$TMPROOT/bin"
WHEEL='fathomdb-0.8.26+tegra-cp310-abi3-linux_aarch64.whl'
OLD_WHEEL='fathomdb-0.8.24+tegra-cp310-abi3-linux_aarch64.whl'
NEW_WHEEL='fathomdb-0.8.27+tegra-cp310-abi3-linux_aarch64.whl'
NEW_TGZ='fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz'
WHEEL_SHA="$(printf '%064d' 0 | tr 0 a)"
OLD_WHEEL_SHA="$(printf '%064d' 0 | tr 0 b)"
NEW_WHEEL_SHA="$(printf '%064d' 0 | tr 0 c)"
NEW_TGZ_SHA="$(printf '%064d' 0 | tr 0 d)"

printf '{"schema":"fathomdb.tegra-pages-retained/v1","artifacts":[{"kind":"wheel","version":"0.8.26+tegra","filename":"%s","sha256":"%s","url":"%s"}]}\n' \
  "$WHEEL" "$WHEEL_SHA" "$SITE/tegra/packages/$WHEEL" >"$TMPROOT/manifest.json"

# live_site WHEEL_LINKS NODE_LINKS: each argument is a space-separated list of
# FILENAME=SHA256 links; '-' leaves that index page absent (HTTP 404).
live_site() {
  local entry
  rm -rf "$FIXSITE"
  mkdir -p "$FIXSITE/.codes"
  if [ "$1" != - ]; then
    mkdir -p "$FIXSITE/tegra/simple/fathomdb"
    {
      printf '<!doctype html>\n<html><body>\n'
      for entry in $1; do
        printf '<a href="../../packages/%s#sha256=%s">%s</a><br>\n' "${entry%%=*}" "${entry#*=}" "${entry%%=*}"
      done
      printf '</body></html>\n'
    } >"$FIXSITE/tegra/simple/fathomdb/index.html"
  fi
  if [ "$2" != - ]; then
    mkdir -p "$FIXSITE/tegra/node"
    {
      printf '<!doctype html>\n<html><body>\n'
      for entry in $2; do
        printf '<a href="../packages/%s#sha256=%s">%s</a><br>\n' "${entry%%=*}" "${entry#*=}" "${entry%%=*}"
      done
      printf '</body></html>\n'
    } >"$FIXSITE/tegra/node/index.html"
  fi
}
# Queue HTTP codes for a page: each fetch of that page consumes one line, and
# once the queue is empty the page is served normally.
queue_codes() {
  local key="$1"
  shift
  printf '%s\n' "$@" >"$FIXSITE/.codes/$key"
}

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
key="$(printf '%s' "$path" | tr '/' '_')"
queue="$CURL_SITE/.codes/$key"
if [ -s "$queue" ]; then
  code="$(head -n 1 "$queue")"
  tail -n +2 "$queue" >"$queue.rest"
  cat "$queue.rest" >"$queue"
  : >"$output"
elif [ -f "$CURL_SITE/$path" ]; then
  cp "$CURL_SITE/$path" "$output"
  code=200
else
  : >"$output"
  code=404
fi
[ -z "$write_out" ] || printf '%s' "$code"
SH
chmod +x "$TMPROOT/bin/curl"
WHEEL_PAGE_KEY='tegra_simple_fathomdb_index.html'

run_check() {
  : >"$TMPROOT/curl.log"
  : >"$TMPROOT/summary.md"
  set +e
  CURL_SITE="$FIXSITE" CURL_LOG="$TMPROOT/curl.log" GITHUB_STEP_SUMMARY="$TMPROOT/summary.md" \
    PATH="$TMPROOT/bin:$PATH" \
    bash "$CHECK" --manifest "$TMPROOT/manifest.json" --retry-interval-seconds 0 "$@" >"$TMPROOT/check.out" 2>&1
  local rc=$?
  set -e
  return "$rc"
}
expect_pass() {
  local label="$1"
  shift
  if run_check "$@"; then pass "$label"; else fail "$label"; cat "$TMPROOT/check.out" >&2; fi
}
expect_fail() {
  local label="$1" needle="$2"
  shift 2
  if run_check "$@"; then
    fail "$label (check passed)"
  elif grep -Fq -- "$needle" "$TMPROOT/check.out"; then
    pass "$label"
  else
    fail "$label (diagnostic lacks: $needle)"
    cat "$TMPROOT/check.out" >&2
  fi
}

live_site "$WHEEL=$WHEEL_SHA" -
expect_pass 'a live site holding only retained artifacts may be redeployed'
if grep -Fq "$SITE/tegra/simple/fathomdb/" "$TMPROOT/curl.log" && grep -Fq "$SITE/tegra/node/" "$TMPROOT/curl.log"; then
  pass 'both live Tegra index pages are fetched'
else
  fail 'both live Tegra index pages must be fetched'
  cat "$TMPROOT/curl.log" >&2
fi
unrestricted="$(grep -v -- "--proto =https" "$TMPROOT/curl.log" || true)"
if grep -q -- "--proto =https" "$TMPROOT/curl.log" && [ -z "$unrestricted" ]; then
  pass 'every live fetch is restricted to https'
else
  fail 'every curl call must pass --proto =https'
  cat "$TMPROOT/curl.log" >&2
fi
expect_pass 'an empty withdrawal input withdraws nothing' --withdraw ''

# The 0.8.24+tegra erasure: a live wheel the retained list does not name.
live_site "$WHEEL=$WHEEL_SHA $OLD_WHEEL=$OLD_WHEEL_SHA" -
expect_fail 'a live wheel missing from the retained list blocks the deployment' "$OLD_WHEEL"
expect_fail 'the refusal says the deployment would withdraw it' 'withdraw'
expect_pass 'naming that wheel as a deliberate withdrawal lets the deployment proceed' --withdraw "$OLD_WHEEL"
if grep -Fq "$OLD_WHEEL" "$TMPROOT/summary.md" && grep -qi 'withdr' "$TMPROOT/summary.md"; then
  pass 'a deliberate withdrawal is recorded in the run summary'
else
  fail 'a deliberate withdrawal must be recorded in the run summary'
  cat "$TMPROOT/summary.md" >&2
fi
expect_fail 'withdrawing a retained artifact is contradictory and refused' 'retained' \
  --withdraw "$OLD_WHEEL,$WHEEL"
expect_fail 'withdrawing an artifact that is not live is refused' 'not linked' \
  --withdraw "$OLD_WHEEL $NEW_WHEEL"
expect_fail 'a path-bearing withdrawal name is a usage error' 'withdraw' \
  --withdraw "../$OLD_WHEEL"

live_site "$WHEEL=$WHEEL_SHA $OLD_WHEEL=$OLD_WHEEL_SHA" "$NEW_TGZ=$NEW_TGZ_SHA"
expect_fail 'a live Node tarball missing from the retained list blocks the deployment' "$NEW_TGZ" \
  --withdraw "$OLD_WHEEL"
expect_pass 'two comma-separated withdrawals are both honoured' --withdraw "$OLD_WHEEL,$NEW_TGZ"
live_site "$WHEEL=$WHEEL_SHA $NEW_WHEEL=$NEW_WHEEL_SHA" "$NEW_TGZ=$NEW_TGZ_SHA"
expect_pass "artifacts this run adds count as retained" \
  --added "$NEW_WHEEL=$NEW_WHEEL_SHA" --added "$NEW_TGZ=$NEW_TGZ_SHA"
expect_fail 'an added artifact replacing a live one under the same name is refused' 'SHA-256' \
  --added "$NEW_WHEEL=$OLD_WHEEL_SHA" --added "$NEW_TGZ=$NEW_TGZ_SHA"
live_site "$WHEEL=$OLD_WHEEL_SHA" -
expect_fail 'a live link whose digest differs from the retained digest is refused' 'SHA-256'

live_site "$WHEEL=$WHEEL_SHA" -
printf '<a href="https://example.invalid/x.whl">x</a>\n' >>"$FIXSITE/tegra/simple/fathomdb/index.html"
expect_fail 'an unrecognised live link fails closed' 'unrecognised'

# Absent index pages: the retained list is the record of what has been
# published, so an absent page is the first-deploy case only while the list
# holds nothing of that kind.
live_site "$WHEEL=$WHEEL_SHA" -
expect_pass 'an absent Node page is the first Node deployment while no tarball is retained'
if grep -qi 'not published' "$TMPROOT/summary.md"; then
  pass 'the first-deploy absent index is recorded in the run summary'
else
  fail 'the first-deploy absent index must be recorded in the run summary'
  cat "$TMPROOT/summary.md" >&2
fi
live_site - -
expect_fail 'an absent wheel page while a wheel is retained blocks the deployment' 'HTTP 404'

# Bounded retries cover transient failures, never an absent page.
live_site "$WHEEL=$WHEEL_SHA" -
queue_codes "$WHEEL_PAGE_KEY" 503 000
expect_pass 'transient server and connection failures are retried' --max-attempts 3
fetches="$(grep -cF "$SITE/tegra/simple/fathomdb/" "$TMPROOT/curl.log" || true)"
if [ "$fetches" = 3 ]; then pass 'a page is refetched only until it answers'; else fail "expected 3 wheel page fetches, saw $fetches"; fi
queue_codes "$WHEEL_PAGE_KEY" 503 503 503 503
expect_fail 'a page still failing after the bounded attempts blocks the deployment' 'HTTP 503' --max-attempts 3
fetches="$(grep -cF "$SITE/tegra/simple/fathomdb/" "$TMPROOT/curl.log" || true)"
if [ "$fetches" = 3 ]; then pass 'retries stop at the attempt bound'; else fail "expected 3 bounded fetches, saw $fetches"; fi
queue_codes "$WHEEL_PAGE_KEY" 403
expect_fail 'an unexpected status blocks the deployment without retrying' 'HTTP 403' --max-attempts 3
expect_fail 'an unbounded attempt count is refused' 'max-attempts' --max-attempts 1000

printf '%s passed, %s failed\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
