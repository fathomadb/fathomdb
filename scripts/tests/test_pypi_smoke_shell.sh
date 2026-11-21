#!/usr/bin/env bash
# Shell control-flow regression; real registry consumers remain release proof.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin"
cat > "$fixture/bin/uname" <<'SH'
#!/usr/bin/env bash
printf 'Darwin\n'
SH
cat > "$fixture/bin/pip" <<'SH'
#!/usr/bin/env bash
exit "${FIXTURE_INSTALL_EXIT:-0}"
SH
cat > "$fixture/bin/python3" <<'SH'
#!/usr/bin/env bash
set -eu
if [ "$1" = -m ]; then
  mkdir -p "$3/bin"
  printf ':\n' > "$3/bin/activate"
  cp "$FIXTURE_BIN/python3" "$3/bin/python"
  exit 0
fi
if [ "$1" = -I ]; then
  printf '%s argc=%s\n' "$5" "$#" >> "$FIXTURE_CALLS"
  if [ "$5" = "${FIXTURE_FAIL_PHASE:-none}" ]; then exit 43; fi
else
  printf 'trace\n' >> "$FIXTURE_CALLS"
  if [ "${FIXTURE_FAIL_PHASE:-none}" = trace ]; then exit 44; fi
fi
SH
chmod +x "$fixture/bin/"*
failed=0
for phase in none write reopen trace install; do
  : > "$fixture/calls"
  status=0
  install_exit=0
  if [ "$phase" = install ]; then install_exit=42; fi
  PATH="$fixture/bin:$PATH" FIXTURE_BIN="$fixture/bin" FIXTURE_CALLS="$fixture/calls" \
    FIXTURE_FAIL_PHASE="$phase" FIXTURE_INSTALL_EXIT="$install_exit" \
    bash "$ROOT/scripts/release/smoke/smoke-pypi-wheel.sh" 0.8.30 > "$fixture/log" 2>&1 || status=$?
  expected=0
  case "$phase" in install) expected=42;; write|reopen) expected=43;; trace) expected=44;; esac
  if [ "$status" -ne "$expected" ]; then
    printf 'FAIL phase=%s expected_exit=%s actual_exit=%s\n' "$phase" "$expected" "$status" >&2
    cat "$fixture/log" >&2
    failed=1
  fi
  expected_calls=''
  case "$phase" in
    none|trace) expected_calls="$(printf 'write argc=5\nreopen argc=5\ntrace')";;
    write) expected_calls='write argc=5';;
    reopen) expected_calls="$(printf 'write argc=5\nreopen argc=5')";;
  esac
  if [ "$(cat "$fixture/calls")" != "$expected_calls" ]; then
    printf 'FAIL phase=%s: required consumer calls absent or extra arguments\n' "$phase" >&2
    cat "$fixture/calls" >&2
    failed=1
  fi
  if [ "$status" -ne 0 ] && grep -q 'smoke-pypi-wheel: ok' "$fixture/log"; then
    printf 'FAIL phase=%s: false success output\n' "$phase" >&2
    failed=1
  fi
done
[ "$failed" -eq 0 ]
printf 'PASS PyPI shell write/reopen/trace execution and original failure exits (Bash %s)\n' "$BASH_VERSION"
