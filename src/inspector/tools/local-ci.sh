#!/usr/bin/env bash
# Local source + native Ubuntu 24.04 package + installed checks. No remote CI claim.
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
root="$(cd -- "$script_dir/../../.." && pwd -P)"
usage() {
  echo 'usage: local-ci.sh [--scoped | --full-workspace | --installed-only] [--base <ref>] [ABSOLUTE_CANDIDATE ABSOLUTE_FIXTURES ABSOLUTE_BUILD_TARGET]' >&2
  exit 2
}

mode=""
mode_count=0
base_ref=""
positional=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --scoped)
      mode="scoped"
      mode_count=$((mode_count + 1))
      shift
      ;;
    --full-workspace)
      mode="full-workspace"
      mode_count=$((mode_count + 1))
      shift
      ;;
    --installed-only)
      mode="installed-only"
      mode_count=$((mode_count + 1))
      shift
      ;;
    --base)
      if [[ $# -lt 2 || -z "$2" ]]; then
        usage
      fi
      base_ref="$2"
      shift 2
      ;;
    --base=*)
      base_ref="${1#--base=}"
      if [[ -z "$base_ref" ]]; then
        usage
      fi
      shift
      ;;
    --)
      shift
      while [[ $# -gt 0 ]]; do
        positional+=("$1")
        shift
      done
      ;;
    --*)
      echo "unknown option: $1" >&2
      usage
      ;;
    *)
      positional+=("$1")
      shift
      ;;
  esac
done

if [[ "$mode_count" -gt 1 ]]; then
  echo 'mode flags --scoped, --full-workspace, and --installed-only are mutually exclusive' >&2
  usage
fi
if [[ -z "$mode" ]]; then
  mode="scoped"
fi
if [[ -z "$base_ref" ]]; then
  base_ref="${INSPECTOR_SCOPED_BASE:-origin/main}"
fi

auto_tmp=""
installed_stage=""
cleanup() {
  if [[ -n "$installed_stage" ]]; then
    rm -rf -- "$installed_stage"
  fi
  if [[ -n "$auto_tmp" ]]; then
    rm -rf -- "$auto_tmp"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if [[ "${#positional[@]}" -eq 3 ]]; then
  candidate="${positional[0]}"
  fixtures="${positional[1]}"
  build_target="${positional[2]}"
  if [[ "$candidate" != /* || "$fixtures" != /* || "$build_target" != /* ]]; then
    usage
  fi
elif [[ "${#positional[@]}" -eq 0 && "$mode" != "installed-only" ]]; then
  auto_tmp="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-inspector-ci.XXXXXX")"
  candidate="$auto_tmp/candidate"
  fixtures="${INSPECTOR_FIXTURE_DIR:-$auto_tmp/fixtures}"
  build_target="${CARGO_TARGET_DIR:-$auto_tmp/target}"
  if [[ "$build_target" == "$auto_tmp/target" ]]; then
    mkdir -p -- "$build_target"
  fi
else
  usage
fi

fail_prerequisite() { echo "prerequisite: $*" >&2; exit 2; }
[[ -d "$fixtures" ]] || fail_prerequisite 'fixtures directory is missing'
for fixture in person vector f06 study-training study-unseen lexical-deadline; do
  [[ -f "$fixtures/$fixture/work/fixture.sqlite" ]] || fail_prerequisite "fixtures/$fixture/work/fixture.sqlite is missing"
done
[[ -d "$build_target" ]] || fail_prerequisite 'build target directory is missing'
[[ -x "$root/.venv/bin/python" ]] || fail_prerequisite 'checkout-owned .venv/bin/python is missing'
python="$root/.venv/bin/python"
"$python" -c 'import sys; assert sys.version_info >= (3, 12)' || fail_prerequisite 'checkout Python >=3.12 is required'
cargo_toml="$root/src/inspector/Cargo.toml"
[[ -f "$cargo_toml" ]] || fail_prerequisite 'src/inspector/Cargo.toml is missing'
version="$("$python" - "$cargo_toml" <<'PY'
import re
import sys

in_pkg = False
for line in open(sys.argv[1], encoding='utf-8'):
    s = line.strip()
    if s.startswith('['):
        in_pkg = s == '[package]'
    elif in_pkg:
        m = re.match(r'^version\s*=\s*"([^"]+)"', s)
        if m:
            print(m.group(1))
            sys.exit(0)
sys.exit('missing [package].version in src/inspector/Cargo.toml')
PY
)"
[[ -n "$version" ]] || fail_prerequisite 'Inspector version could not be derived from src/inspector/Cargo.toml'
bundle_name="fathomdb-inspector-${version}-linux-x86_64"

for tool in git cargo node Xvfb flock readelf; do
  command -v "$tool" >/dev/null || fail_prerequisite "$tool is missing"
done
browser="${INSPECTOR_CHROMIUM_DIRECTORY:-$HOME/.cache/ms-playwright/chromium-1248/chrome-linux64}"
modules="${INSPECTOR_NODE_MODULES:-$HOME/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules}"
[[ -x "$browser/chrome" ]] || fail_prerequisite 'installed Chromium executable is missing'
[[ -f "$modules/playwright/index.mjs" ]] || fail_prerequisite 'existing Playwright modules are missing'
cd -- "$root"

if [[ "$mode" == "scoped" ]]; then
  if ! git rev-parse --verify --quiet "${base_ref}^{commit}" >/dev/null 2>&1; then
    echo "scoped diff guard: base ref '$base_ref' is unresolvable; pass --base <ref> or run --full-workspace" >&2
    exit 2
  fi
  committed_diff="$(git diff --name-only "${base_ref}...HEAD")"
  working_diff="$(git status --porcelain)"
  "$python" - "$base_ref" "$committed_diff" "$working_diff" <<'PY'
import sys

base_ref, committed, working = sys.argv[1], sys.argv[2], sys.argv[3]
allowed = ('src/inspector/', 'dev/inspector/', 'docs/inspector/')
paths = []
for line in committed.splitlines():
    entry = line.strip()
    if entry:
        paths.append(entry)
for line in working.splitlines():
    if len(line) < 4:
        continue
    entry = line[3:]
    if ' -> ' in entry:
        old, new = entry.split(' -> ', 1)
        paths.extend([old.strip('"'), new.strip('"')])
    else:
        paths.append(entry.strip('"'))
out_of_scope = sorted({p for p in paths if not p.startswith(allowed)})
if out_of_scope:
    print(
        f'scoped diff guard failed against {base_ref}: changes outside src/inspector/, dev/inspector/, docs/inspector/:',
        file=sys.stderr,
    )
    for path in out_of_scope:
        print(f'  {path}', file=sys.stderr)
    print(
        'Run with --full-workspace (or --base <ref> for branch-local Inspector commits).',
        file=sys.stderr,
    )
    sys.exit(1)
PY
fi

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
if [[ "$mode" != "installed-only" ]]; then
  [[ ! -e "$candidate" && ! -L "$candidate" ]] || fail_prerequisite 'candidate output must be new'
  candidate_parent="$(dirname -- "$candidate")"
  parent="$(realpath -e -- "$candidate_parent")"
  candidate_leaf="$(basename -- "$candidate")"
  [[ "$parent/$candidate_leaf" == "$candidate" ]] || fail_prerequisite 'candidate parent must be canonical'
  [[ "$candidate" != "$root/dist/"* ]] || fail_prerequisite 'candidate must stay outside final dist'
  if [[ "$mode" == "scoped" ]]; then
    scoped_source() {
      bash scripts/agent-lint.sh || return $?
      bash scripts/agent-typecheck.sh || return $?
      cargo test --locked --offline -p fathomdb-inspector --jobs 1 -- --test-threads=1 || return $?
      cargo test --locked --offline -p fathomdb-inspector --no-default-features --features cached-cpu --jobs 1 -- --test-threads=1 || return $?
    }
    stage source scoped_source
  else
    stage source bash scripts/agent-verify.sh
  fi
  stage tooling "$python" -m unittest discover -s src/inspector/tests/tools -p 'test_*.py'
  stage unit node --test src/inspector/tests/unit/comparison.mjs src/inspector/tests/unit/ux.mjs
  checked_sha="$(git rev-parse HEAD)"
  source_status="$(git status --porcelain)"
  [[ "$source_sha" == "$checked_sha" && -z "$source_status" ]] || fail_prerequisite 'source changed during checks'
  native_build() {
    local glibc_version
    glibc_version="$(getconf GNU_LIBC_VERSION)" || return $?
    [[ "$glibc_version" == "glibc 2.39" ]] || return 1
    CARGO_TARGET_DIR="$build_target" bash src/inspector/tools/package.sh "$candidate" || return $?
    cargo build --locked --offline --release --target x86_64-unknown-linux-gnu --target-dir "$build_target" --no-default-features --features cached-cpu -p fathomdb-inspector --examples || return $?
  }
  stage native-build native_build
  stage seal "$python" src/inspector/tools/local-release.py seal "$candidate"
fi
stage candidate "$python" src/inspector/tools/local-release.py verify "$candidate" --source
stage abi "$python" - "$candidate/$bundle_name/fathomdb-inspector" <<'PY'
import re
import subprocess
import sys
versions = re.findall(r'GLIBC_(\d+)\.(\d+)', subprocess.check_output(['readelf', '--version-info', sys.argv[1]], text=True))
maximum = max(tuple(map(int, version)) for version in versions)
assert maximum <= (2, 39), f'ELF requires GLIBC {maximum}, above qualified Ubuntu 24.04 userland'
print(f'ELF maximum GLIBC {maximum[0]}.{maximum[1]}')
PY
installed_stage="$(mktemp -d "${TMPDIR:-/tmp}/inspector-ci-installed.XXXXXX")"
stage extract tar -xzf "$candidate/$bundle_name.tar.gz" -C "$installed_stage"
stage installed bash src/inspector/tools/run-installed.sh \
  "$installed_stage/$bundle_name" "$fixtures" "$build_target" \
  --test --test-concurrency=1 src/inspector/tests/browser/installed.mjs \
  src/inspector/tests/browser/attributes.mjs \
  src/inspector/tests/browser/comparison.mjs src/inspector/tests/browser/lexical.mjs \
  src/inspector/tests/browser/vector.mjs src/inspector/tests/browser/graph.mjs \
  src/inspector/tests/browser/workspace.mjs src/inspector/tests/browser/ux.mjs
if [[ "$mode" == "installed-only" ]]; then
  echo 'local-ci PASS installed-only (source gate not run)'
elif [[ "$mode" == "scoped" ]]; then
  echo "local-ci PASS mode=scoped base=$base_ref source=$source_sha; source(scoped)/tooling/unit/native-build/seal/candidate/abi/installed (not a full-workspace claim)"
else
  echo "local-ci PASS mode=full-workspace source=$source_sha; source/tooling/unit/native-build/seal/candidate/abi/installed"
fi
