#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
VERIFY="$ROOT/scripts/verify-release-python-wheel.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

if [ ! -x "$VERIFY" ]; then
  fail "release wheel verifier is missing or not executable"
fi

mkdir -p "$TMP/bin" "$TMP/primary/fathomdb"
cat >"$TMP/bin/maturin" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--out" ]; then out="$2"; shift 2; else shift; fi
done
if [ "${FAKE_NO_WHEEL:-0}" != 1 ]; then
  mkdir -p "$out"
  : >"$out/fathomdb-0.8.25-cp312-abi3-linux_x86_64.whl"
fi
SH
chmod +x "$TMP/bin/maturin"

cat >"$TMP/fake-python" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = "-m" ] && [ "${2:-}" = "venv" ]; then
  venv="$3"
  mkdir -p "$venv/bin" "$venv/lib/python3.12/site-packages/fathomdb"
  cp "$0" "$venv/bin/python"
  exit 0
fi
if [ "${1:-}" = "-m" ] && [ "${2:-}" = "pip" ]; then exit 0; fi
if [ -n "${FATHOMDB_VERIFY_REPORT:-}" ]; then
  venv="${FAKE_VENV:?}"
  module="${FAKE_MODULE:-$venv/lib/python3.12/site-packages/fathomdb/__init__.py}"
  native="${FAKE_NATIVE:-$venv/lib/python3.12/site-packages/fathomdb/_fathomdb.so}"
  printf '%s\n%s\n%s\n' "$module" "$native" "${FAKE_EDITABLE:-false}" >"$FATHOMDB_VERIFY_REPORT"
  printf 'wheel smoke: ok\n'
  exit 0
fi
exit 0
SH
chmod +x "$TMP/fake-python"

run_case() {
  name="$1"; shift
  wheel="$TMP/wheel-$name"
  venv="$TMP/venv-$name"
  set +e
  output="$(env PATH="$TMP/bin:$PATH" FAKE_VENV="$venv" "$@" \
    "$VERIFY" --python "$TMP/fake-python" --wheel-dir "$wheel" --venv-dir "$venv" 2>&1)"
  rc=$?
  set -e
}

run_case success env
[ "$rc" -eq 0 ] || fail "valid isolated wheel fixture passes: $output"
pass "valid isolated wheel fixture passes"

run_case missing env FAKE_NO_WHEEL=1
[ "$rc" -ne 0 ] && grep -q 'exactly one wheel' <<<"$output" \
  || fail "missing wheel is rejected: $output"
pass "missing wheel is rejected"

run_case primary-module env FAKE_MODULE="$TMP/primary/fathomdb/__init__.py"
[ "$rc" -ne 0 ] && grep -q 'module escaped fresh venv' <<<"$output" \
  || fail "primary-tree Python module is rejected: $output"
pass "primary-tree Python module is rejected"

run_case primary-native env FAKE_NATIVE="$TMP/primary/fathomdb/_fathomdb.so"
[ "$rc" -ne 0 ] && grep -q 'native module escaped fresh venv' <<<"$output" \
  || fail "primary-tree native module is rejected: $output"
pass "primary-tree native module is rejected"

run_case editable env FAKE_EDITABLE=true
[ "$rc" -ne 0 ] && grep -q 'editable install' <<<"$output" \
  || fail "editable installation is rejected: $output"
pass "editable installation is rejected"

printf '\nAll release wheel verifier tests passed\n'
