#!/usr/bin/env bash
# Contract tests for P7-08: CHANGELOG section extraction and github-release wiring.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
EXTRACTOR="$REPO_ROOT/scripts/release/extract-changelog-section.sh"
RELEASE_WF="$REPO_ROOT/.github/workflows/release.yml"
PROMOTE_WF="$REPO_ROOT/.github/workflows/promote-npm-release.yml"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [ ! -x "$EXTRACTOR" ]; then
  printf 'FAIL missing executable extractor: %s\n' "$EXTRACTOR" >&2
  exit 1
fi

cat >"$WORK/CHANGELOG.md" <<'MD'
# Changelog

## Unreleased

## 0.8.31 — 2026-11-22

### Added

- Inline typed properties.

## v0.8.30 - 2026-11-21

### Fixed

- Older release body.

## 0.8.29

## 0.8.28 — 2026-11-10

- Earlier note.
MD

# 1. Exact version match without leading v
bash "$EXTRACTOR" 0.8.31 "$WORK/CHANGELOG.md" "$WORK/out-31.md"
grep -qF '## 0.8.31 — 2026-11-22' "$WORK/out-31.md"
grep -qF -- '- Inline typed properties.' "$WORK/out-31.md"
if grep -qF '0.8.30' "$WORK/out-31.md"; then
  printf 'FAIL extractor leaked next release heading into 0.8.31 body\n' >&2
  exit 1
fi
printf 'PASS extracts 0.8.31 section bounded by next ## heading\n'

# 2. Exact version match with leading v in heading
bash "$EXTRACTOR" 0.8.30 "$WORK/CHANGELOG.md" "$WORK/out-30.md"
grep -qF '## v0.8.30 - 2026-11-21' "$WORK/out-30.md"
grep -qF -- '- Older release body.' "$WORK/out-30.md"
if grep -qF '0.8.29' "$WORK/out-30.md"; then
  printf 'FAIL extractor leaked 0.8.29 into 0.8.30 body\n' >&2
  exit 1
fi
printf 'PASS extracts v-prefixed heading\n'

# 3. Empty section must fail closed
if bash "$EXTRACTOR" 0.8.29 "$WORK/CHANGELOG.md" "$WORK/out-29.md" 2>/dev/null; then
  printf 'FAIL empty changelog section was accepted\n' >&2
  exit 1
fi
printf 'PASS rejects empty changelog section\n'

# 4. Missing version must fail closed
if bash "$EXTRACTOR" 9.9.9 "$WORK/CHANGELOG.md" "$WORK/out-missing.md" 2>/dev/null; then
  printf 'FAIL missing changelog version was accepted\n' >&2
  exit 1
fi
printf 'PASS rejects missing changelog version\n'

# 5. Real repository CHANGELOG.md contains 0.8.30
bash "$EXTRACTOR" 0.8.30 "$REPO_ROOT/CHANGELOG.md" "$WORK/out-real.md"
grep -qF '## 0.8.30' "$WORK/out-real.md"
printf 'PASS extracts real 0.8.30 section from repo CHANGELOG.md\n'

# 6. release.yml and promote-npm-release.yml github-release jobs wire checkout + body_path
python3 - "$RELEASE_WF" "$PROMOTE_WF" <<'PY'
import sys, yaml

release_wf = yaml.safe_load(open(sys.argv[1]))
gr = release_wf["jobs"]["github-release"]
steps = gr["steps"]
checkouts = [s for s in steps if str(s.get("uses", "")).startswith("actions/checkout@")]
assert len(checkouts) == 1, f"release.yml github-release needs checkout: {checkouts}"
assert checkouts[0]["with"]["ref"] == "${{ env.RELEASE_CHECKOUT_REF }}"
assert checkouts[0]["with"]["persist-credentials"] is False
assert any("extract-changelog-section.sh" in s.get("run", "") for s in steps), "missing extractor step in release.yml"
gh_rel = [s for s in steps if str(s.get("uses", "")).startswith("softprops/action-gh-release@")][0]
assert gh_rel["with"].get("body_path") == "${{ runner.temp }}/release-body.md", gh_rel["with"]

promote_wf = yaml.safe_load(open(sys.argv[2]))
pgr = promote_wf["jobs"]["github-release"]
psteps = pgr["steps"]
pcheckouts = [s for s in psteps if str(s.get("uses", "")).startswith("actions/checkout@")]
assert len(pcheckouts) == 1, f"promote-npm-release.yml github-release needs checkout: {pcheckouts}"
assert pcheckouts[0]["with"]["ref"] == "${{ github.workflow_sha }}"
assert pcheckouts[0]["with"]["persist-credentials"] is False
assert any("extract-changelog-section.sh" in s.get("run", "") for s in psteps), "missing extractor step in promote-npm-release.yml"
pgh_rel = [s for s in psteps if str(s.get("uses", "")).startswith("softprops/action-gh-release@")][0]
assert pgh_rel["with"].get("body_path") == "${{ runner.temp }}/release-body.md", pgh_rel["with"]
PY
printf 'PASS release.yml and promote-npm-release.yml github-release wire CHANGELOG body_path\n'
