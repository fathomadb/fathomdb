#!/usr/bin/env bash
# Contract for the documented, exact-SHA Tegra Pages publication path.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PUBLISHER="$REPO_ROOT/scripts/release/publish-tegra-pages.sh"
RUNBOOK="$REPO_ROOT/docs/operations/tegra-pages-publication.md"
PROMPT="$REPO_ROOT/dev/plans/prompts/0.8.26-TEGRA-PAGES-PUBLICATION.md"
failed=0

require_file() {
  local path="$1" label="$2"
  if [ -f "$path" ]; then printf 'PASS  %s\n' "$label"; else printf 'FAIL  %s\n' "$label" >&2; failed=1; fi
}
require_text() {
  local path="$1" needle="$2" label="$3"
  if grep -Fq -- "$needle" "$path"; then printf 'PASS  %s\n' "$label"; else printf 'FAIL  %s\n' "$label" >&2; failed=1; fi
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
fi
if [ -f "$RUNBOOK" ]; then
  require_text "$RUNBOOK" 'publish-tegra-pages.sh' 'runbook names the one-command publisher'
  require_text "$RUNBOOK" '0.8.26+tegra' 'runbook names the exact Tegra distribution'
  require_text "$RUNBOOK" 'Maturin 1.14.1' 'runbook records the pinned build tool'
  require_text "$RUNBOOK" 'not published to PyPI' 'runbook states the registry boundary'
fi
if [ -f "$PROMPT" ]; then
  require_text "$PROMPT" 'publish-tegra-pages.sh' 'handoff uses the publisher script'
  require_text "$PROMPT" 'Do not move' 'handoff preserves the existing tag'
  require_text "$PROMPT" 'Troubleshoot' 'handoff includes a repair loop'
fi

[ "$failed" -eq 0 ]
