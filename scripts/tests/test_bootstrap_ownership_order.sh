#!/usr/bin/env bash
# Ownership refusals must precede every mutating bootstrap invocation.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mode="${1:-venv}"
case "$mode" in venv|modules) ;; *) exit 2 ;; esac
failed=0
for caller in bootstrap.sh bootstrap-heavy.sh; do
  targets=(.venv)
  if [ "$mode" = modules ]; then targets=(node_modules src/ts/node_modules); fi
  for target in "${targets[@]}"; do
    fixture="$(mktemp -d "$work/fixture with spaces.XXXXXX")"
    foreign="$(mktemp -d "$work/foreign.XXXXXX")"
    bin="$fixture/bin"
    calls="$fixture/calls"
    mkdir -p "$bin" "$fixture/scripts/lib" "$fixture/src/python" "$fixture/src/ts" "$(dirname "$fixture/$target")"
    cp "$root/scripts/$caller" "$root/scripts/setup-python-owned.sh" "$fixture/scripts/"
    cp "$root/scripts/lib/agent-python-env.sh" "$fixture/scripts/lib/"
    cp "$root/src/python/pyproject.toml" "$fixture/src/python/"
    printf '{}\n' > "$fixture/src/ts/package.json"
    printf '{}\n' > "$fixture/package.json"
    ln -s "$foreign" "$fixture/$target"
    git -C "$fixture" init -q
    git -C "$fixture" -c user.name=Fixture -c user.email=fixture@example.invalid commit --allow-empty -qm fixture
    for installer in install-gitleaks.sh install-hooks.sh install-actionlint.sh install-shellcheck.sh; do
      # shellcheck disable=SC2016
      printf '#!/usr/bin/env bash\nprintf "installer|%%s\\n" "%s" >> "$ORDER_CALLS"\n' "$installer" > "$fixture/scripts/$installer"
      chmod +x "$fixture/scripts/$installer"
    done
    cat > "$bin/python3.13" <<'PYTHON'
#!/usr/bin/env bash
set -euo pipefail
[ "$1 $2" = '-m venv' ] || exit 91
printf 'venv\n' >> "$ORDER_CALLS"
mkdir -p "$3/bin"
printf mutation > "$3/foreign-mutation"
cat > "$3/bin/python" <<'VENV'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = - ]; then
  shift
  exec /usr/bin/python3 -c 'import os,sys; sys.prefix=os.environ["ORDER_FIXTURE"]+"/.venv"; exec(sys.stdin.read())' "$@"
fi
printf 'python-mutator\n' >> "$ORDER_CALLS"
VENV
chmod +x "$3/bin/python"
PYTHON
    cat > "$bin/npm" <<'NPM'
#!/usr/bin/env bash
set -euo pipefail
printf 'npm\n' >> "$ORDER_CALLS"
if [ -L "$PWD/node_modules" ]; then printf mutation > "$PWD/node_modules/foreign-mutation"; fi
NPM
    chmod +x "$bin/python3.13" "$bin/npm"
    set +e
    (cd "$fixture"; env VIRTUAL_ENV= PATH="$bin:$PATH" ORDER_CALLS="$calls" ORDER_FIXTURE="$fixture" bash "scripts/$caller") > "$fixture/output" 2>&1
    rc=$?
    set -e
    if [ "$rc" -eq 0 ] || ! grep -Fq 'setup-python-owned:' "$fixture/output"; then
      cat "$fixture/output"
      printf 'FAIL %s did not issue the ownership refusal for %s\n' "$caller" "$target" >&2
      failed=1
    fi
    if [ -e "$foreign/foreign-mutation" ]; then
      printf 'FAIL %s mutated foreign %s before refusal\n' "$caller" "$target" >&2
      failed=1
    fi
    if [ -s "$calls" ]; then
      cat "$calls"
      printf 'FAIL %s invoked mutator before ownership refusal for %s\n' "$caller" "$target" >&2
      failed=1
    fi
  done
done
[ "$failed" -eq 0 ]
printf 'PASS both bootstrap callers reject foreign %s before mutation or installers\n' "$mode"
