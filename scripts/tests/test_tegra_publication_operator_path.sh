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
