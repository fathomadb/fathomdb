#!/usr/bin/env bash
# Regression/mutation controls for documentation targets and publication truth.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="${CHECKER_UNDER_TEST:-$REPO_ROOT/scripts/check-public-doc-truth.py}"
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT
FIXTURE="$TMPROOT/fixture"

make_fixture() {
  rm -rf "$FIXTURE"
  mkdir -p "$FIXTURE/scripts" "$FIXTURE/dev/plans/runs" "$FIXTURE/docs/generated"
  python3 - "$REPO_ROOT" "$FIXTURE" <<'PY'
import ast
from pathlib import Path
import shutil
import sys
root, fixture = map(Path, sys.argv[1:])
m = ast.parse((root / 'scripts/check-public-doc-truth.py').read_text())
a = next(n for n in m.body if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'PUBLIC_DOCS' for t in n.targets))
for e in a.value.elts:
    p = Path(e.args[0].value)
    dest = fixture / p
    dest.parent.mkdir(parents=True, exist_ok=True)
    # A historical checkout remains a valid no-marker fallback case.
    text = (root / p).read_text().replace('0.8.27', '0.8.26')
    text += '\nCurrent published release: v0.8.26 is published.\n'
    text += '\nNative platforms: Linux x86_64, Linux AArch64, macOS x64/arm64, and Windows x64.\n'
    if p.as_posix() == 'docs/install/typescript.md':
        text = text.replace('npm install --save-exact fathomdb@0.8.26', 'npm install fathomdb@next')
        text += '\nRecorded published route: `npm install fathomdb@latest`.\n'
    if p.as_posix() == 'README.md':
        text += '\n- Twelve Rust workspace members\n'
    dest.write_text(text)
for p in ['Cargo.toml', 'scripts/release-current.py', 'dev/platform-capabilities.json', 'dev/plans/release-state-0.8.26.json', 'dev/plans/runs/STATUS-0.8.26.md', 'docs/generated/contributor-map.md']:
    shutil.copyfile(root / p, fixture / p)
PY
  git -C "$FIXTURE" init -q
  git -C "$FIXTURE" add .
}

pending_fixture() {
  make_fixture
  python3 - "$REPO_ROOT" "$FIXTURE" <<'PY'
import ast
import json
from pathlib import Path
import sys
root, fixture = map(Path, sys.argv[1:])
m = ast.parse((root / 'scripts/check-public-doc-truth.py').read_text())
a = next(n for n in m.body if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'PUBLIC_DOCS' for t in n.targets))
for e in a.value.elts:
    p = Path(e.args[0].value)
    text = (root / p).read_text()
    text = text.replace('The published 0.8.27 line', 'The 0.8.27 line')
    text = text.replace('Install the published 0.8.27 CLI', 'Install the 0.8.27 CLI')
    (fixture / p).write_text(text)
state = {'release': '0.8.27', 'board': 'dev/plans/runs/STATUS-0.8.27.md', 'schema_version': 34, 'public_docs_target_release': '0.8.27', 'published': None}
(fixture / 'dev/plans/release-state-0.8.27.json').write_text(json.dumps(state) + '\n')
(fixture / state['board']).write_text('# Active release 0.8.27\n')
PY
  git -C "$FIXTURE" add .
}

mutate_state() {
  python3 - "$FIXTURE/dev/plans/release-state-$1.json" "$2" "$3" <<'PY'
import json
from pathlib import Path
import sys
p = Path(sys.argv[1]); s = json.loads(p.read_text()); s[sys.argv[2]] = json.loads(sys.argv[3]); p.write_text(json.dumps(s) + '\n')
PY
}
expect_pass() {
  if REPO_ROOT="$FIXTURE" python3 "$CHECKER" >"$TMPROOT/output" 2>&1; then
    printf 'PASS  %s\n' "$1"
  else
    cat "$TMPROOT/output" >&2
    printf 'FAIL  %s\n' "$1" >&2; exit 1
  fi
}
expect_fail() {
  if REPO_ROOT="$FIXTURE" python3 "$CHECKER" >"$TMPROOT/output" 2>&1; then
    printf 'FAIL  %s\n' "$1" >&2; exit 1
  fi
  if ! grep -Eq "$2" "$TMPROOT/output"; then
    cat "$TMPROOT/output" >&2
    printf 'FAIL  wrong diagnostic: %s\n' "$1" >&2; exit 1
  fi
  printf 'PASS  %s\n' "$1"
}

make_fixture
expect_pass 'historical published 0.8.26 fallback remains valid'
make_fixture
cp "$REPO_ROOT/dev/plans/release-state-0.8.22.json" "$FIXTURE/dev/plans/"
cp "$REPO_ROOT/dev/plans/runs/STATUS-0.8.22.md" "$FIXTURE/dev/plans/runs/"
git -C "$FIXTURE" add .
expect_pass 'newest valid tracked published record wins'
pending_fixture
expect_pass 'pending 0.8.27 target accepts publication-facing docs without publication claim'
pending_fixture
mutate_state 0.8.27 schema_version 35
expect_fail 'selected active target schema must be 34 independently of historical state' 'documentation target.*schema 34'
pending_fixture
mutate_state 0.8.26 schema_version 35
expect_fail 'published schema guard remains independent of pending target' 'published release state.*schema 34'
pending_fixture
mutate_state 0.8.27 published '{"tag":"v0.8.27","tag_commit":"1111111111111111111111111111111111111111","published_on":"2026-10-10","npm_dist_tag":"latest"}'
expect_pass 'publication receipt accepts unchanged publication-facing 0.8.27 docs'
pending_fixture
mutate_state 0.8.27 public_docs_target_release '"0.8.28"'
expect_fail 'foreign documentation target fails closed' 'public_docs_target_release.*own release'
pending_fixture
mutate_state 0.8.27 public_docs_target_release 827
expect_fail 'non-string documentation target fails closed' 'public_docs_target_release'
pending_fixture
mutate_state 0.8.27 public_docs_target_release null
expect_fail 'explicit null documentation target fails closed' 'public_docs_target_release'
pending_fixture
python3 - "$FIXTURE/docs/reference/python-api.md" <<'PY'
from pathlib import Path
import sys
p=Path(sys.argv[1]);p.write_text(p.read_text().replace('0.8.27', '0.8.25'))
PY
expect_fail 'missing target version in API landing fails' 'python-api.md.*documentation target'
pending_fixture
sed -i 's/Release 0.8.27/Release 0.8.25/' "$FIXTURE/docs/reference/python-api.md"
expect_fail 'source-link version cannot mask a stale API release label' 'python-api.md.*documentation target'
pending_fixture
sed -i 's/npm install --save-exact fathomdb@0.8.27/npm install --save-exact fathomdb@0.8.25/' "$FIXTURE/docs/install/typescript.md"
expect_fail 'pending target requires exact npm install route' 'typescript.md.*exact.*0.8.27'
for suffix in rc.1 malicious; do
  pending_fixture
  sed -i "s/fathomdb@0.8.27/fathomdb@0.8.27-$suffix/g" "$FIXTURE/docs/install/typescript.md"
  expect_fail "npm target suffix $suffix cannot pass as exact release" 'typescript.md.*exact.*0.8.27'
done
for claim in 'Current published FathomDB 0.8.27.' 'FathomDB 0.8.27 is available on PyPI now.' '0.8.27 tarball is published on Pages.' '0.8.27 wheels have shipped.' 'FathomDB 0.8.27 is now published on PyPI.' 'The 0.8.27 release is now available on PyPI.'; do
  pending_fixture
  printf '\n%s\n' "$claim" >>"$FIXTURE/README.md"
  expect_fail "pending publication claim: $claim" 'README.md.*unpublished.*published'
done
pending_fixture
printf '\nFathomDB 0.8.27 is already published.\n' >>"$FIXTURE/README.md"
expect_fail 'pending target cannot assert publication' 'README.md.*unpublished.*published'
pending_fixture
printf '\nThe published 0.8.27 release is available.\n' >>"$FIXTURE/docs/index.md"
expect_fail 'published-version adjective also fails for pending target' 'docs/index.md.*unpublished.*published'
pending_fixture
printf '\nHistorical note: 0.8.26 is published; 0.8.19 was not yet published then.\n' >>"$FIXTURE/docs/index.md"
expect_pass 'historical publication prose is not a pending target claim'
make_fixture
python3 - "$FIXTURE" <<'PY'
import json
from pathlib import Path
import sys
r=Path(sys.argv[1]);s={'release':'0.8.27','board':'dev/plans/runs/STATUS-0.8.27.md','schema_version':34,'published':None};(r/'dev/plans/release-state-0.8.27.json').write_text(json.dumps(s));(r/s['board']).write_text('# Active release\n')
PY
git -C "$FIXTURE" add .
expect_pass 'active release without marker falls back to published docs'
make_fixture
mutate_state 0.8.26 public_docs_target_release '"9.9.9"'
expect_pass 'historical marker does not override the active target or fallback'
make_fixture
python3 - "$FIXTURE/dev/plans/release-state-9.9.9.json" <<'PY'
from pathlib import Path
import json,sys
Path(sys.argv[1]).write_text(json.dumps({'release':'9.9.9','public_docs_target_release':'9.9.9','published':None}))
PY
expect_pass 'untracked future target cannot alter public truth'
pending_fixture
cp "$FIXTURE/dev/plans/release-state-0.8.27.json" "$FIXTURE/dev/plans/release-state-0.8.28.json"
sed -i 's/0.8.27/0.8.28/g' "$FIXTURE/dev/plans/release-state-0.8.28.json"
printf '# Active release\n' >"$FIXTURE/dev/plans/runs/STATUS-0.8.28.md"
git -C "$FIXTURE" add .
expect_fail 'ambiguous canonical active release fails closed' 'expected exactly one tracked live release'
pending_fixture
mutate_state 0.8.27 release_kind '"released; publication complete"'
expect_fail 'publication-complete lifecycle without receipt fails closed' 'publication complete without a published receipt'
make_fixture
mutate_state 0.8.26 published '{"tag":"v0.8.24","tag_commit":"0000000000000000000000000000000000000000","published_on":"2026-08-31","npm_dist_tag":"latest"}'
expect_fail 'malformed published tag remains rejected' 'published tag.*does not match'
make_fixture
python3 - "$FIXTURE/dev/plans/release-state-0.8.26.json" <<'PY'
import json,sys
from pathlib import Path
p=Path(sys.argv[1]);s=json.loads(p.read_text());s['published']['published_on']='2026-02-30';p.write_text(json.dumps(s))
PY
expect_fail 'invalid published calendar date remains rejected' 'published_on.*ISO date'
make_fixture
sed -i 's/v0.8.26 is published/v0.8.26 is not yet published/' "$FIXTURE/README.md"
expect_fail 'unpublished claim for a published version remains rejected' 'README.md.*published 0.8.26 is unpublished'
pending_fixture
sed -i 's/| \*\*yes\*\* (manylinux 2_28) |/| no |/' "$FIXTURE/docs/compatibility/index.md"
expect_fail 'published aarch64 manifest/platform proof remains required' 'aarch64-unknown-linux-gnu.*table row'
pending_fixture
python3 - "$FIXTURE/dev/platform-capabilities.json" <<'PY2'
import json,sys
from pathlib import Path
p=Path(sys.argv[1]);m=json.loads(p.read_text());m['platforms'][0]['status']='unpublished';p.write_text(json.dumps(m))
PY2
expect_fail 'published native manifest proof remains required' 'five published native artifacts'
pending_fixture
sed -i 's/, macOS x64\/arm64//' "$FIXTURE/docs/index.md"
expect_fail 'macOS boundary remains required' 'docs/index.md.*macOS'
pending_fixture
sed -i 's/, and Windows x64//' "$FIXTURE/docs/getting-started/index.md"
expect_fail 'Windows boundary remains required' 'getting-started/index.md.*Windows'
pending_fixture
sed -i 's/SCHEMA_VERSION` to \*\*34\*\*/SCHEMA_VERSION` to **35**/' "$FIXTURE/docs/compatibility/index.md"
expect_fail 'schema fresh-database boundary remains required' 'schema-34 fresh-database'
pending_fixture
sed -i 's/no public automatic migration/automatic migration/' "$FIXTURE/docs/compatibility/index.md"
expect_fail 'no automatic migration boundary remains required' 'schema-34 fresh-database'
pending_fixture
printf '\n- Nine Rust workspace members\n' >>"$FIXTURE/README.md"
expect_fail 'false optional README crate count remains rejected' 'claims 9 Rust workspace members'
pending_fixture
sed -i '/^| `fathomdb-sdk` |/d' "$FIXTURE/docs/generated/contributor-map.md"
expect_fail 'generated contributor workspace coverage cannot lose a member' 'contributor-map.md.*workspace'
make_fixture
sed -i -E 's/npm install fathomdb@(next|latest)/npm install fathomdb@0.8.23/g' "$FIXTURE/docs/install/typescript.md"
expect_fail 'published fallback requires exact version or recorded npm tag' 'typescript.md.*0.8.26.*latest'
printf '\nAll public-doc-truth tests passed\n'
