#!/usr/bin/env bash
# Local source + native Ubuntu 24.04 package + installed checks. No remote CI claim.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
installed_only=0
if [[ ${1:-} == --installed-only ]]; then
  installed_only=1
  shift
fi
if [[ $# != 3 || "$1" != /* || "$2" != /* || "$3" != /* ]]; then
  echo 'usage: local-ci.sh [--installed-only] ABSOLUTE_CANDIDATE ABSOLUTE_FIXTURES ABSOLUTE_BUILD_TARGET' >&2
  exit 2
fi
candidate="$1"
fixtures="$2"
build_target="$3"
fail_prerequisite() { echo "prerequisite: $*" >&2; exit 2; }
[[ -d "$fixtures" ]] || fail_prerequisite 'fixtures directory is missing'
for fixture in person vector f06 study-training study-unseen lexical-deadline; do
  [[ -f "$fixtures/$fixture/work/fixture.sqlite" ]] || fail_prerequisite "fixtures/$fixture/work/fixture.sqlite is missing"
done
[[ -d "$build_target" ]] || fail_prerequisite 'build target directory is missing'
[[ -x "$root/.venv/bin/python" ]] || fail_prerequisite 'checkout-owned .venv/bin/python is missing'
python="$root/.venv/bin/python"
"$python" -c 'import sys; assert sys.version_info >= (3, 12)' || fail_prerequisite 'checkout Python >=3.12 is required'
for tool in git cargo node Xvfb flock readelf; do
  command -v "$tool" >/dev/null || fail_prerequisite "$tool is missing"
done
browser="${INSPECTOR_CHROMIUM_DIRECTORY:-$HOME/.cache/ms-playwright/chromium-1248/chrome-linux64}"
modules="${INSPECTOR_NODE_MODULES:-$HOME/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules}"
[[ -x "$browser/chrome" ]] || fail_prerequisite 'installed Chromium executable is missing'
[[ -f "$modules/playwright/index.mjs" ]] || fail_prerequisite 'existing Playwright modules are missing'
cd -- "$root"
source_status="$(git status --porcelain)"
[[ -z "$source_status" ]] || fail_prerequisite 'source checkout must be clean'
source_sha="$(git rev-parse HEAD)"
stage() {
  local name="$1" status
  shift
  echo "local-ci stage=$name" >&2
  if "$@"; then
    return 0
  else
    status=$?
    echo "local-ci FAIL stage=$name exit=$status" >&2
    return "$status"
  fi
}
if [[ "$installed_only" == 0 ]]; then
  [[ ! -e "$candidate" && ! -L "$candidate" ]] || fail_prerequisite 'candidate output must be new'
  parent="$(realpath -e -- "$(dirname -- "$candidate")")"
  [[ "$parent/$(basename -- "$candidate")" == "$candidate" ]] || fail_prerequisite 'candidate parent must be canonical'
  [[ "$candidate" != "$root/dist/"* ]] || fail_prerequisite 'candidate must stay outside final dist'
  stage source bash scripts/agent-verify.sh
  stage tooling "$root/.venv/bin/python" -m unittest discover -s src/inspector/tests/tools -p 'test_*.py'
  stage unit node --test src/inspector/tests/unit/comparison.mjs src/inspector/tests/unit/ux.mjs
  checked_sha="$(git rev-parse HEAD)"
  source_status="$(git status --porcelain)"
  [[ "$source_sha" == "$checked_sha" && -z "$source_status" ]] || fail_prerequisite 'source changed during checks'
  native_build() {
    local glibc_version
    glibc_version="$(getconf GNU_LIBC_VERSION)"
    [[ "$glibc_version" == "glibc 2.39" ]]
    CARGO_TARGET_DIR="$build_target" bash src/inspector/tools/package.sh "$candidate"
    cargo build --locked --offline --release --target x86_64-unknown-linux-gnu --target-dir "$build_target" --no-default-features --features cached-cpu -p fathomdb-inspector --examples
  }
  stage native-build native_build
  stage seal "$python" src/inspector/tools/local-release.py seal "$candidate"
fi
stage candidate "$python" src/inspector/tools/local-release.py verify "$candidate" --source
stage abi "$python" - "$candidate/fathomdb-inspector-0.2.1-linux-x86_64/fathomdb-inspector" <<'PY'
import re
import subprocess
import sys
versions = re.findall(r'GLIBC_(\d+)\.(\d+)', subprocess.check_output(['readelf', '--version-info', sys.argv[1]], text=True))
maximum = max(tuple(map(int, version)) for version in versions)
assert maximum <= (2, 39), f'ELF requires GLIBC {maximum}, above qualified Ubuntu 24.04 userland'
print(f'ELF maximum GLIBC {maximum[0]}.{maximum[1]}')
PY
installed_stage="$(mktemp -d "${TMPDIR:-/tmp}/inspector-ci-installed.XXXXXX")"
cleanup_installed() { rm -rf -- "$installed_stage"; }
trap cleanup_installed EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
stage extract tar -xzf "$candidate/fathomdb-inspector-0.2.1-linux-x86_64.tar.gz" -C "$installed_stage"
stage installed bash src/inspector/tools/run-installed.sh \
  "$installed_stage/fathomdb-inspector-0.2.1-linux-x86_64" "$fixtures" "$build_target" \
  --test --test-concurrency=1 src/inspector/tests/browser/installed.mjs \
  src/inspector/tests/browser/comparison.mjs src/inspector/tests/browser/lexical.mjs \
  src/inspector/tests/browser/vector.mjs src/inspector/tests/browser/graph.mjs \
  src/inspector/tests/browser/workspace.mjs src/inspector/tests/browser/ux.mjs
if [[ "$installed_only" == 1 ]]; then
  echo 'local-ci PASS installed-only (source gate not run)'
else
  echo "local-ci PASS source=$source_sha; source/tooling/unit/native-build/seal/candidate/abi/installed"
fi
