#!/usr/bin/env bash
# Focused regression coverage for the public current-truth guard.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="${CHECKER_UNDER_TEST:-$REPO_ROOT/scripts/check-public-doc-truth.py}"

TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

make_fixture() {
  local root="$1"
  rm -rf "$root"
  mkdir -p "$root/dev/plans" "$root/docs/getting-started" \
    "$root/docs/install" "$root/docs/compatibility"
  cp "$REPO_ROOT/README.md" "$root/README.md"
  cp "$REPO_ROOT/Cargo.toml" "$root/Cargo.toml"
  cp "$REPO_ROOT/dev/plans/release-state-0.8.23.json" "$root/dev/plans/"
  cp "$REPO_ROOT/dev/platform-capabilities.json" "$root/dev/"
  cp "$REPO_ROOT/docs/index.md" "$root/docs/"
  cp "$REPO_ROOT/docs/getting-started/index.md" "$root/docs/getting-started/"
  cp "$REPO_ROOT/docs/install/"{python,typescript,rust}.md "$root/docs/install/"
  cp "$REPO_ROOT/docs/compatibility/index.md" "$root/docs/compatibility/"
  git -C "$root" init -q
  git -C "$root" add .
}

expect_pass() {
  local root="$1" description="$2"
  if REPO_ROOT="$root" python3 "$CHECKER" >/dev/null; then
    printf 'PASS  %s\n' "$description"
  else
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
}

expect_fail() {
  local root="$1" description="$2"
  if REPO_ROOT="$root" python3 "$CHECKER" >/dev/null 2>&1; then
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
  printf 'PASS  %s\n' "$description"
}

FIXTURE="$TMPROOT/fixture"
make_fixture "$FIXTURE"
expect_pass "$FIXTURE" 'baseline current public facts agree with release and platform state'

make_fixture "$FIXTURE"
cp "$REPO_ROOT/dev/plans/release-state-0.8.22.json" "$FIXTURE/dev/plans/"
git -C "$FIXTURE" add dev/plans/release-state-0.8.22.json
expect_pass "$FIXTURE" 'newest valid tracked published state wins over an older state'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/dev/plans/release-state-0.8.24.json" <<'PY'
import json
from pathlib import Path
import sys

state = {
    "release": "0.8.24",
    "board": "dev/plans/runs/STATUS-0.8.24.md",
    "published": None,
}
Path(sys.argv[1]).write_text(json.dumps(state) + "\n")
PY
git -C "$FIXTURE" add dev/plans/release-state-0.8.24.json
expect_pass "$FIXTURE" 'newer unpublished state is ignored for public release truth'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/dev/plans/release-state-9.9.9.json" <<'PY'
import json
from pathlib import Path
import sys

state = {
    "release": "9.9.9",
    "release_kind": "released; publication complete",
    "board": "dev/plans/runs/STATUS-9.9.9.md",
    "published": {
        "tag": "v9.9.9",
        "tag_commit": "0" * 40,
        "published_on": "2026-08-31",
        "npm_dist_tag": "latest",
    },
}
Path(sys.argv[1]).write_text(json.dumps(state) + "\n")
PY
expect_pass "$FIXTURE" 'untracked future state cannot alter public release truth'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/dev/plans/release-state-0.8.23.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
state = json.loads(path.read_text())
state["published"] = {
    "tag": "v0.8.22",
    "tag_commit": "0" * 40,
    "published_on": "2026-08-31",
    "npm_dist_tag": "latest",
}
path.write_text(json.dumps(state) + "\n")
PY
expect_fail "$FIXTURE" 'rejects a malformed published tag record'

sed -i 's/v0\.8\.23 is published/v0.8.23 is not yet published/' "$FIXTURE/README.md"
expect_fail "$FIXTURE" 'rejects an unpublished claim for the published release'

make_fixture "$FIXTURE"
sed -i 's/aarch64 is published/aarch64 is not published/' "$FIXTURE/docs/compatibility/index.md"
expect_fail "$FIXTURE" 'requires the published aarch64 native-artifact assertion'

make_fixture "$FIXTURE"
sed -i 's/npm install fathomdb@next/npm install fathomdb@0.8.23/' "$FIXTURE/docs/install/typescript.md"
expect_fail "$FIXTURE" 'requires npm next install guidance'

make_fixture "$FIXTURE"
sed -i 's/Ten Rust workspace members/Nine Rust workspace members/' "$FIXTURE/README.md"
expect_fail "$FIXTURE" 'rejects a false Rust workspace member count'

make_fixture "$FIXTURE"
printf '\nHistorical note: 0.8.19 is not yet published in this example.\n' >>"$FIXTURE/docs/index.md"
expect_pass "$FIXTURE" 'does not treat historical-version prose as a current-release claim'

printf '\nAll public-doc-truth tests passed\n'
