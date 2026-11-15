#!/usr/bin/env bash
# Exercise existing setup callers with controlled installer/build processes.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fixture="$work/repo with spaces"
mkdir -p "$fixture/src/python" "$fixture/src/ts" "$fixture/src/rust/crates/fathomdb-py" "$work/bin" "$fixture/.venv/bin"
cp "$root/src/python/pyproject.toml" "$fixture/src/python/"
cp "$root/src/rust/crates/fathomdb-py/Cargo.toml" "$fixture/src/rust/crates/fathomdb-py/"
printf '{}\n' > "$fixture/src/ts/package.json"
git -C "$fixture" init -q
git -C "$fixture" -c user.name=Fixture -c user.email=fixture@example.invalid commit --allow-empty -qm fixture
export S10_FIXTURE="$fixture" S10_CALLS="$work/calls"
cat > "$work/python" <<'PY'
#!/usr/bin/env bash
set -euo pipefail
printf 'python' >> "$S10_CALLS"; printf '|%s' "$@" >> "$S10_CALLS"; printf '\n' >> "$S10_CALLS"
if [ "${1:-}" = - ]; then
  shift
  exec /usr/bin/python3 -c 'import os,sys; sys.prefix=os.environ.get("S10_CONTROLLER_PREFIX",os.environ["S10_FIXTURE"]+"/.venv"); exec(sys.stdin.read())' "$@"
fi
if [ "${1:-}" = -m ] && [ "${2:-}" = pip ]; then
  [ "${3:-}" = install ] || exit 0
  [ "${S10_FAIL_INSTALL:-0}" = 0 ] || exit 71
  for pair in 'ruff|ruff 0.15.17' 'pyright|pyright 1.1.410'; do
    printf '#!/usr/bin/env bash\nprintf "%s\\n"\n' "${pair#*|}" > "$S10_FIXTURE/.venv/bin/${pair%%|*}"
    chmod +x "$S10_FIXTURE/.venv/bin/${pair%%|*}"
  done
elif [ "${1:-}" = -m ] && [ "${2:-}" = maturin ]; then
  while [ "$#" -gt 0 ]; do if [ "$1" = --out ]; then mkdir -p "$2"; printf fixture > "$2/fathomdb-fixture.whl"; break; fi; shift; done
elif [ "${1:-}" = -c ] && [[ "${2:-}" == *'S10_NATIVE_PROBE'* ]]; then
  /usr/bin/python3 - <<'PROBE'
import hashlib,json,os
from pathlib import Path
v=Path(os.environ['S10_FIXTURE'])/'.venv';p=v/'lib/native.so';p.parent.mkdir(exist_ok=True);p.write_bytes(b'native-fixture')
print(json.dumps({'prefix':str(v),'module':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'python':'fixture','abi':'fixture'}))
PROBE
fi
PY
chmod +x "$work/python"
cp "$work/python" "$fixture/.venv/bin/python"
for pair in 'ruff|ruff 0.15.17' 'pyright|pyright 1.1.410'; do
  printf '#!/usr/bin/env bash\nprintf "%s\\n"\n' "${pair#*|}" > "$fixture/.venv/bin/${pair%%|*}"
  chmod +x "$fixture/.venv/bin/${pair%%|*}"
done
cat > "$work/bin/python3.13" <<'PY'
#!/usr/bin/env bash
set -euo pipefail
[ "$1 $2" = '-m venv' ] || exit 91
mkdir -p "$3/bin"; cp "$S10_FAKE_PYTHON" "$3/bin/python"
PY
cat > "$work/bin/cargo" <<'SH'
#!/usr/bin/env bash
printf 'cargo|%s\n' "$*" >> "$S10_CALLS"
SH
cat > "$work/bin/npm" <<'SH'
#!/usr/bin/env bash
printf 'npm|%s|%s\n' "$PWD" "$*" >> "$S10_CALLS"
SH
printf '#!/usr/bin/env bash\nprintf "1.7.12\\n"\n' > "$work/bin/actionlint"
printf '#!/usr/bin/env bash\nprintf "version: 0.11.0\\n"\n' > "$work/bin/shellcheck"
chmod +x "$work/bin/"*
export S10_FAKE_PYTHON="$work/python"
export PATH="$work/bin:$PATH"
assert_build_selection() {
  /usr/bin/python3 - "$1" "$2" <<'PY'
import json,os,sys
from pathlib import Path
features,compatibility=sys.argv[1:]
calls=Path(os.environ['S10_CALLS']).read_text().splitlines()
builds=[line.split('|')[1:] for line in calls if line.startswith('python|-m|maturin|build|')]
assert len(builds)==1, builds
args=builds[0]
assert args[:4]==['-m','maturin','build','--locked'], args
assert '--release' in args, args
assert args.count('--features')==1 and args[args.index('--features')+1]==features, args
if compatibility:
    assert args.count('--compatibility')==1 and args[args.index('--compatibility')+1]==compatibility, args
else:
    assert '--compatibility' not in args, args
receipt=json.loads((Path(os.environ['S10_FIXTURE'])/'.cache/agent/python-native.json').read_text())
assert receipt['features']==features, receipt
assert receipt['compatibility']==(compatibility or None), receipt
assert any(line.startswith('python|-m|pip|install|--force-reinstall|--no-deps|') for line in calls), calls
assert any('S10_NATIVE_PROBE' in line for line in calls), calls
PY
}
case "${1:-bootstrap}" in
 default)
  (cd "$fixture"; bash "$root/scripts/setup-python-owned.sh")
  assert_build_selection 'pyo3/extension-module,default-embedder,default-reranker' ''
  echo 'PASS default build preserves Pyproject features and compatibility policy'
  ;;
 cuda)
  # shellcheck source=../release/cuda-artifact-contract.sh
  . "$root/scripts/release/cuda-artifact-contract.sh"
  (cd "$fixture"; bash "$root/scripts/setup-python-owned.sh" --build --features "$CUDA_PYTHON_FEATURES" --compatibility linux)
  assert_build_selection 'pyo3/extension-module,embed-cuda' linux
  echo 'PASS CUDA build selects contract features and host compatibility in argv and receipt'
  ;;
 observer)
  (cd "$fixture"; bash "$root/scripts/setup-python-owned.sh" --provision --compatibility linux --features pyo3/extension-module,embed-cuda,rerank-cuda,cuda-pool-observer)
  assert_build_selection 'pyo3/extension-module,embed-cuda,rerank-cuda,cuda-pool-observer' linux
  grep -Fq 'python|-m|pip|install|pytest' "$S10_CALLS"
  echo 'PASS provision supports a separately selected reranking observer counterpart'
  ;;
 refusal)
  refuse() {
    : > "$S10_CALLS"
    if (cd "$fixture"; bash "$root/scripts/setup-python-owned.sh" "$@"); then
      echo "FAIL invalid setup arguments accepted: $*" >&2; exit 1
    fi
    if grep -Eq 'python\|-m\|(maturin\|build|pip\|install)\|' "$S10_CALLS"; then
      echo "FAIL invalid setup arguments reached build/install: $*" >&2; exit 1
    fi
  }
  refuse --provision --features pyo3/extension-module,unknown-feature
  refuse --provision --features pyo3/extension-module,fathomdb-engine/embed-cuda
  refuse --provision --features embed-cuda
  refuse --provision --features pyo3/extension-module,embed-cuda,embed-cuda
  refuse --provision --features pyo3/extension-module,
  refuse --provision --features ,pyo3/extension-module
  refuse --provision --features pyo3/extension-module,,embed-cuda
  refuse --provision --features 'pyo3/extension-module, embed-cuda'
  # shellcheck disable=SC2016 # The unexpanded shell text is the invalid input.
  refuse --provision --features 'pyo3/extension-module,$(touch injected)'
  refuse --provision --features ''
  refuse --provision --features
  refuse --provision --features --compatibility linux
  refuse --provision --features pyo3/extension-module --features pyo3/extension-module
  refuse --provision --compatibility ''
  refuse --provision --compatibility manylinux_2_28
  refuse --provision --compatibility
  refuse --provision --compatibility linux --compatibility linux
  refuse --provision --unknown-option
  refuse --build --provision
  refuse --build extra
  refuse --check-ownership --features pyo3/extension-module
  refuse --check-ownership --compatibility linux
  test ! -e "$fixture/.cache/agent/python-native.json"
  test ! -e "$fixture/injected"
  echo 'PASS malformed/unknown/repeated selections stop before build/install'
  ;;
 bootstrap)
  (cd "$fixture"; bash "$root/scripts/bootstrap-heavy.sh")
  if grep -Fq '|install|-e|' "$S10_CALLS"; then echo 'FAIL setup caller still installs editable Python' >&2; exit 1; fi
  grep -Fq 'python|-m|maturin|build|' "$S10_CALLS" || { echo 'FAIL setup caller did not build a noneditable wheel' >&2; exit 1; }
  test -s "$fixture/.cache/agent/python-native.json"
  echo 'PASS bootstrap installs an owned noneditable wheel'
  ;;
 stale)
  mkdir -p "$fixture/.cache/agent"
  touch "$fixture/.cache/agent/python-installed"
  (cd "$fixture"; bash "$root/scripts/agent-build.sh")
  grep -Fq 'python|-m|maturin|build|' "$S10_CALLS" || { echo 'FAIL stale timestamp incorrectly skipped candidate wheel build' >&2; exit 1; }
  echo 'PASS stale timestamp cannot authorize native reuse'
  ;;
 missing)
  /usr/bin/python3 -c 'import shutil,sys; shutil.rmtree(sys.argv[1])' "$fixture/.venv"
  if (cd "$fixture"; bash "$root/scripts/agent-build.sh"); then echo 'FAIL missing environment accepted' >&2; exit 1; fi
  if [ -e "$S10_CALLS" ] && grep -Fq 'cargo|' "$S10_CALLS"; then echo 'FAIL missing owned environment reached Cargo workload' >&2; exit 1; fi
  echo 'PASS missing owned environment stops before Cargo'
  ;;
 banner)
  (cd "$fixture"; bash "$root/scripts/bootstrap-heavy.sh")
  printf '#!/usr/bin/env bash\nprintf "pyright 1.1.410\\nWARNING: a newer version is available\\n"\n' > "$fixture/.venv/bin/pyright"
  (cd "$fixture"; bash "$root/scripts/setup-python-owned.sh" --build)
  echo 'PASS exact Pyright pin accepts its informational update banner'
  ;;
 foreign)
  if (cd "$fixture"; S10_CONTROLLER_PREFIX=/foreign bash "$root/scripts/setup-python-owned.sh" --provision); then echo 'FAIL foreign interpreter prefix accepted' >&2; exit 1; fi
  if [ -e "$S10_CALLS" ] && grep -Fq '|pip|install|' "$S10_CALLS"; then echo 'FAIL foreign interpreter reached package installation' >&2; exit 1; fi
  echo 'PASS foreign interpreter prefix stops before installation'
  ;;
 *) echo 'unknown setup fixture case'     >&2; exit 2 ;;
esac
