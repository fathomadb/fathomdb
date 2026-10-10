#!/usr/bin/env bash
# Contract for the documented, exact-SHA Tegra Pages publication path.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PUBLISHER="$REPO_ROOT/scripts/release/publish-tegra-pages.sh"
RUNBOOK="$REPO_ROOT/docs/operations/tegra-pages-publication.md"
PROMPT="$REPO_ROOT/dev/plans/prompts/0.8.27-TEGRA-PAGES-PUBLICATION.md"
failed=0

require_file() {
  local path="$1" label="$2"
  if [ -f "$path" ]; then printf 'PASS  %s\n' "$label"; else printf 'FAIL  %s\n' "$label" >&2; failed=1; fi
}
require_text() {
  local path="$1" needle="$2" label="$3"
  if grep -Fq -- "$needle" "$path"; then printf 'PASS  %s\n' "$label"; else printf 'FAIL  %s\n' "$label" >&2; failed=1; fi
}
require_absent() {
  local path="$1" needle="$2" label="$3"
  if grep -Fq -- "$needle" "$path"; then printf 'FAIL  %s\n' "$label" >&2; failed=1; else printf 'PASS  %s\n' "$label"; fi
}

require_file "$PUBLISHER" 'exact-SHA Tegra publisher script exists'
require_file "$RUNBOOK" 'public Tegra publication runbook exists'
require_file "$PROMPT" 'LLM Tegra publication handoff exists'

if [ -f "$PUBLISHER" ]; then
  require_text "$PUBLISHER" 'publish_to_pages=true' 'publisher requires explicit Pages opt-in'
  require_text "$PUBLISHER" 'candidate_sha=' 'publisher binds the requested SHA'
  require_text "$PUBLISHER" 'candidate_version=' 'publisher binds the requested version'
  require_text "$PUBLISHER" 'release/$candidate_version' 'publisher binds the versioned release branch'
  require_text "$PUBLISHER" 'git ls-remote' 'publisher refuses a SHA other than the remote release head'
  require_text "$PUBLISHER" 'gh run watch' 'publisher waits for CI completion'
  require_text "$PUBLISHER" 'smoke-tegra-pages-wheel.sh' 'publisher performs the installed Pages-wheel smoke'
  require_absent "$PUBLISHER" '--event workflow_dispatch' 'publisher uses the supported gh run-list interface'
  require_text "$PUBLISHER" 'smoke-tegra-node-package.sh' 'publisher performs the installed Pages Node smoke'
  require_text "$PUBLISHER" '--source pages' 'publisher installs the Node tarball from Pages, not local bytes'
  require_text "$PUBLISHER" 'node/SHA256SUMS' 'publisher binds the Node tarball to the Jetson build digest'
  require_text "$PUBLISHER" 'tegra-pages-retained.json' 'publisher names the retained list the published artifacts join'
  require_absent "$PUBLISHER" 'npm publish' 'publisher never publishes to the npm registry'
fi
if [ -f "$RUNBOOK" ]; then
  require_text "$RUNBOOK" 'publish-tegra-pages.sh' 'runbook names the one-command publisher'
  require_text "$RUNBOOK" '0.8.27+tegra' 'runbook names the exact Tegra distribution'
  require_text "$RUNBOOK" 'Maturin 1.14.1' 'runbook records the pinned build tool'
  require_text "$RUNBOOK" 'not published to PyPI' 'runbook states the registry boundary'
fi
if [ -f "$PROMPT" ]; then
  require_text "$PROMPT" 'publish-tegra-pages.sh' 'handoff uses the publisher script'
  require_text "$PROMPT" 'Do not move' 'handoff preserves the existing tag'
  require_text "$PROMPT" 'Troubleshoot' 'handoff includes a repair loop'
fi

[ "$failed" -eq 0 ]

# A CI green result without the installed Pages wheel smoke is an incomplete
# publication check, even when an operator deliberately skips that smoke.
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT
mkdir -p "$TMPROOT/bin"
cat > "$TMPROOT/bin/git" <<'MOCK_GIT'
#!/usr/bin/env bash
printf '%s\trefs/heads/release/0.8.27\n' "${MOCK_REMOTE_SHA:-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"
MOCK_GIT
cat > "$TMPROOT/bin/gh" <<'MOCK_GH'
#!/usr/bin/env bash
case "$1 $2" in
  'auth status'|'workflow run'|'run watch') exit 0 ;;
  'run download')
    dir=''
    while [ "$#" -gt 0 ]; do
      case "$1" in
        --dir) dir="$2"; shift 2 ;;
        *) shift ;;
      esac
    done
    mkdir -p "$dir"
    cp -R "$MOCK_EVIDENCE"/. "$dir/" ;;
  'run list')
    if [ ! -e "$MOCK_STATE" ]; then
      : > "$MOCK_STATE"
      printf '[]\n'
    else
      printf '[{"databaseId":123,"headBranch":"release/0.8.27","headSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","status":"completed","conclusion":"success","url":"https://example.invalid/run/123"}]\n'
    fi ;;
  'run view')
    printf '{"headSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","status":"completed","conclusion":"%s","url":"https://example.invalid/run/123"}\n' "${MOCK_VIEW_CONCLUSION:-success}" ;;
  *) printf 'unexpected gh call: %s\n' "$*" >&2; exit 1 ;;
esac
MOCK_GH
chmod +x "$TMPROOT/bin/git" "$TMPROOT/bin/gh"
set +e
MOCK_STATE="$TMPROOT/state" PATH="$TMPROOT/bin:$PATH" "$PUBLISHER" \
  --candidate-version 0.8.27 \
  --candidate-sha aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
  --skip-pages-smoke > "$TMPROOT/skip.out" 2>&1
skip_rc=$?
set -e
if [ "$skip_rc" -ne 0 ] && grep -Fq 'INCOMPLETE' "$TMPROOT/skip.out"; then
  printf 'PASS  explicit smoke skip cannot report completed publication\n'
else
  printf 'FAIL  explicit smoke skip reported success: rc=%s\n' "$skip_rc" >&2
  cat "$TMPROOT/skip.out" >&2
  exit 1
fi

set +e
MOCK_REMOTE_SHA=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb \
  MOCK_STATE="$TMPROOT/state" PATH="$TMPROOT/bin:$PATH" "$PUBLISHER" \
  --candidate-version 0.8.27 \
  --candidate-sha aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
  --skip-pages-smoke > "$TMPROOT/mismatch.out" 2>&1
mismatch_rc=$?
set -e
if [ "$mismatch_rc" -ne 0 ] && grep -Fq 'not requested' "$TMPROOT/mismatch.out"; then
  printf 'PASS  mismatched remote release SHA fails before dispatch\n'
else
  printf 'FAIL  mismatched remote SHA was accepted\n' >&2
  exit 1
fi

rm -f "$TMPROOT/state"
set +e
MOCK_VIEW_CONCLUSION=failure MOCK_STATE="$TMPROOT/state" PATH="$TMPROOT/bin:$PATH" "$PUBLISHER" \
  --candidate-version 0.8.27 \
  --candidate-sha aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
  --skip-pages-smoke > "$TMPROOT/unverified.out" 2>&1
unverified_rc=$?
set -e
if [ "$unverified_rc" -ne 0 ] && grep -Fq 'CI did not succeed' "$TMPROOT/unverified.out"; then
  printf 'PASS  unverified workflow result fails before publication claim\n'
else
  printf 'FAIL  unverified workflow result was accepted\n' >&2
  exit 1
fi

# --- Node tarball negative cases ------------------------------------------------
# The publisher runs from a stand-in checkout whose smoke scripts record their
# arguments, so these cases exercise the real publisher without a Jetson.
FAKE_ROOT="$TMPROOT/checkout"
mkdir -p "$FAKE_ROOT/scripts/release/smoke"
cp "$PUBLISHER" "$FAKE_ROOT/scripts/release/publish-tegra-pages.sh"
cat > "$FAKE_ROOT/scripts/release/smoke/smoke-tegra-pages-wheel.sh" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$MOCK_CALLS/wheel-smoke.args"
exit "${MOCK_WHEEL_SMOKE_RC:-0}"
STUB
cat > "$FAKE_ROOT/scripts/release/smoke/smoke-tegra-node-package.sh" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$MOCK_CALLS/node-smoke.args"
exit "${MOCK_NODE_SMOKE_RC:-0}"
STUB
WHEEL_DIGEST='1111111111111111111111111111111111111111111111111111111111111111'
NODE_DIGEST='2222222222222222222222222222222222222222222222222222222222222222'
make_evidence() {
  local dir="$1" node_line="$2"
  mkdir -p "$dir/wheel" "$dir/node"
  printf '%s  %s\n' "$WHEEL_DIGEST" "$dir/wheel/fathomdb-0.8.27+tegra-cp310-abi3-linux_aarch64.whl" > "$dir/wheel.sha256"
  : > "$dir/wheel/fathomdb-0.8.27+tegra-cp310-abi3-linux_aarch64.whl"
  if [ -n "$node_line" ]; then
    printf '%s\n' "$node_line" > "$dir/node/SHA256SUMS"
  fi
}
run_publisher() {
  local label="$1" evidence="$2"
  shift 2
  rm -rf "$TMPROOT/state" "$TMPROOT/calls"
  mkdir -p "$TMPROOT/calls"
  set +e
  (cd "$FAKE_ROOT" && env MOCK_STATE="$TMPROOT/state" MOCK_EVIDENCE="$evidence" MOCK_CALLS="$TMPROOT/calls" \
    PATH="$TMPROOT/bin:$PATH" "$@" bash scripts/release/publish-tegra-pages.sh \
    --candidate-version 0.8.27 --candidate-sha aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) \
    > "$TMPROOT/$label.out" 2>&1
  local rc=$?
  set -e
  printf '%s\n' "$rc" > "$TMPROOT/$label.rc"
}
expect_not_published() {
  local label="$1" needle="$2" description="$3" rc
  rc="$(cat "$TMPROOT/$label.rc")"
  if [ "$rc" -ne 0 ] && grep -Fq -- "$needle" "$TMPROOT/$label.out" \
    && ! grep -Fq 'published and smoke-tested' "$TMPROOT/$label.out"; then
    printf 'PASS  %s\n' "$description"
  else
    printf 'FAIL  %s (rc=%s)\n' "$description" "$rc" >&2
    cat "$TMPROOT/$label.out" >&2
    exit 1
  fi
}

make_evidence "$TMPROOT/ev-no-node" ''
run_publisher no-node "$TMPROOT/ev-no-node" env
expect_not_published no-node 'no valid Node tarball SHA-256' \
  'evidence without a Node tarball digest fails before any smoke'
if [ -e "$TMPROOT/calls/wheel-smoke.args" ] || [ -e "$TMPROOT/calls/node-smoke.args" ]; then
  printf 'FAIL  no smoke may run without both retained digests\n' >&2
  exit 1
fi
printf 'PASS  no smoke runs without both retained digests\n'

make_evidence "$TMPROOT/ev-bad-node" "not-a-digest  fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz"
run_publisher bad-node "$TMPROOT/ev-bad-node" env
expect_not_published bad-node 'no valid Node tarball SHA-256' 'a malformed Node tarball digest fails'

make_evidence "$TMPROOT/ev-other-node" "$NODE_DIGEST  fathomdb-linux-arm64-gnu-tegra-0.8.26.tgz"
run_publisher other-node "$TMPROOT/ev-other-node" env
expect_not_published other-node 'no valid Node tarball SHA-256' \
  'a digest for another version of the Node tarball is not accepted'

make_evidence "$TMPROOT/ev-good" "$NODE_DIGEST  fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz"
run_publisher node-fails "$TMPROOT/ev-good" env MOCK_NODE_SMOKE_RC=1
expect_not_published node-fails 'INCOMPLETE' 'a failed Pages Node smoke reports INCOMPLETE, not publication'

run_publisher wheel-fails "$TMPROOT/ev-good" env MOCK_WHEEL_SMOKE_RC=1
expect_not_published wheel-fails 'INCOMPLETE' 'a failed Pages wheel smoke reports INCOMPLETE, not publication'

run_publisher good "$TMPROOT/ev-good" env
if [ "$(cat "$TMPROOT/good.rc")" -eq 0 ] && grep -Fq 'published and smoke-tested' "$TMPROOT/good.out"; then
  printf 'PASS  both Pages smokes passing completes the publication check\n'
else
  printf 'FAIL  passing Pages smokes did not complete the publication check\n' >&2
  cat "$TMPROOT/good.out" >&2
  exit 1
fi
node_args="$(cat "$TMPROOT/calls/node-smoke.args")"
for expected in '--source' 'pages' '--version' '0.8.27' \
  'https://fathomadb.github.io/fathomdb/tegra/packages/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz' \
  "$NODE_DIGEST" '--pages-run' '123' 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'; do
  if ! grep -Fxq -- "$expected" <<<"$node_args"; then
    printf 'FAIL  Pages Node smoke must receive %s; got:\n%s\n' "$expected" "$node_args" >&2
    exit 1
  fi
done
printf 'PASS  Pages Node smoke receives the exact Pages URL, retained digest, run and candidate\n'
if grep -Fq '"kind": "node-tarball"' "$TMPROOT/good.out" \
  && grep -Fq "\"sha256\": \"$NODE_DIGEST\"" "$TMPROOT/good.out" \
  && grep -Fq '"kind": "wheel"' "$TMPROOT/good.out"; then
  printf 'PASS  publisher prints the retained-list entries for the published pair\n'
else
  printf 'FAIL  publisher must print retained-list entries for the published wheel and tarball\n' >&2
  cat "$TMPROOT/good.out" >&2
  exit 1
fi
