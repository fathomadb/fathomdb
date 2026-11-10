#!/usr/bin/env bash
# Exercise existing setup callers with controlled installer/build processes.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fixture="$work/repo with spaces"
mkdir -p "$fixture/src/python" "$fixture/src/ts" "$work/bin" "$fixture/.venv/bin"
cp "$root/src/python/pyproject.toml" "$fixture/src/python/"
printf '{}\n' > "$fixture/src/ts/package.json"
git -C "$fixture" init -q
git -C "$fixture" -c user.name=Fixture -c user.email=fixture@example.invalid commit --allow-empty -qm fixture
export S10_FIXTURE="$fixture" S10_CALLS="$work/calls"
cat > "$work/python" <<'PY'
#!/usr/bin/env bash
set -euo pipefail
printf 'python' >> "$S10_CALLS"; printf '|%s' "$@" >> "$S10_CALLS"; printf '\n' >> "$S10_CALLS"
if [ "${1:-}" = - ]; then exec /usr/bin/python3 "$@"; fi
if [ "${1:-}" = -m ] && [ "${2:-}" = pip ]; then
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
cat > "$work/bin/python3.13" <<'PY'
#!/usr/bin/env bash
set -euo pipefail
[ "$1 $2" = '-m venv' ] || exit 91
mkdir -p "$3/bin"; cp "$S10_FAKE_PYTHON" "$3/bin/python"
PY
cat > "$work/bin/npm" <<'SH'
#!/usr/bin/env bash
printf 'npm|%s|%s\n' "$PWD" "$*" >> "$S10_CALLS"
SH
printf '#!/usr/bin/env bash\nprintf "1.7.12\\n"\n' > "$work/bin/actionlint"
printf '#!/usr/bin/env bash\nprintf "version: 0.11.0\\n"\n' > "$work/bin/shellcheck"
chmod +x "$work/bin/"*
export S10_FAKE_PYTHON="$work/python"
export PATH="$work/bin:$PATH"
case "${1:-bootstrap}" in
 bootstrap)
  (cd "$fixture"; bash "$root/scripts/bootstrap-heavy.sh")
  if grep -Fq '|install|-e|' "$S10_CALLS"; then echo 'FAIL setup caller still installs editable Python' >&2; exit 1; fi
  grep -Fq 'python|-m|maturin|build|' "$S10_CALLS" || { echo 'FAIL setup caller did not build a noneditable wheel' >&2; exit 1; }
  test -s "$fixture/.cache/agent/python-native.json"
  echo 'PASS bootstrap installs an owned noneditable wheel'
  ;;
 *) echo 'unknown setup fixture case' >&2; exit 2 ;;
esac
