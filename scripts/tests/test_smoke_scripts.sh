#!/usr/bin/env bash
# scripts/tests/test_smoke_scripts.sh — STRUCTURAL coverage for the three
# post-publish smoke scripts under scripts/release/smoke/.
#
# WHY this is structural (no shelled-out integration):
#   The smoke scripts install from real registries (crates.io / PyPI /
#   npm). Running them in unit tests would require network, the published
#   artifact already existing at the test version, and tens of seconds of
#   wall time per script — all flaky in CI. Their behavior is exercised
#   in production at tag time by the release workflow's post-publish-smoke
#   job. What we CAN test cheaply is that the script body has the
#   contract-shape we depend on:
#     * hardened bash (`set -euo pipefail`).
#     * version regex check on $1 BEFORE any registry call.
#     * mktemp -d + EXIT trap for cleanup (no leaked work dirs on failure).
#     * version-pinned install command (the published version is the
#       version under test, not "latest" or "*").
#   These are the structural invariants that, if broken, would cause the
#   real smoke job to install the wrong artifact or leak files.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SMOKE_DIR="$REPO_ROOT/scripts/release/smoke"

FAILED=0
pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

# Asserts a string is present in the file body; fails with the literal
# needle in the diagnostic so debugging is one grep away.
assert_contains() {
  local label="$1" file="$2" needle="$3"
  if grep -qF -- "$needle" "$file"; then
    pass "$label"
  else
    fail "$label (missing literal: $needle)"
  fi
}

assert_matches() {
  local label="$1" file="$2" pattern="$3"
  if grep -qE -- "$pattern" "$file"; then
    pass "$label"
  else
    fail "$label (no match for pattern: $pattern)"
  fi
}

assert_next_line() {
  local label="$1" file="$2" command="$3" expected="$4"
  if awk -v command="$command" -v expected="$expected" '
    $0 == command { getline; if ($0 == expected) { found = 1 } }
    END { exit(found ? 0 : 1) }
  ' "$file"; then
    pass "$label"
  else
    fail "$label (native command is not immediately followed by: $expected)"
  fi
}

check_common() {
  local script="$1" label_prefix="$2"
  [ -x "$script" ] || fail "$label_prefix: not executable"
  # Shebang.
  if head -1 "$script" | grep -qE '^#!.*bash'; then
    pass "$label_prefix: bash shebang"
  else
    fail "$label_prefix: missing bash shebang"
  fi
  assert_contains "$label_prefix: set -euo pipefail" "$script" 'set -euo pipefail'
  # Version regex — SemVer 2.0 (MAJOR.MINOR.PATCH with optional
  # pre-release identifier) guard on $1.
  assert_matches  "$label_prefix: version regex guard on \$1" "$script" \
    '\^\[0-9\]\+\\\.\[0-9\]\+\\\.\[0-9\]\+\(-\[0-9A-Za-z\.-\]\+\)\?\$'
  assert_contains "$label_prefix: mktemp -d for fixture dir" "$script" 'mktemp -d'
  assert_contains "$label_prefix: EXIT trap cleanup" "$script" "trap 'rm -rf"
}

CRATES="$SMOKE_DIR/smoke-crates-cli.sh"
PYPI="$SMOKE_DIR/smoke-pypi-wheel.sh"
NPM="$SMOKE_DIR/smoke-npm-package.sh"
PYPI_WINDOWS="$SMOKE_DIR/smoke-pypi-wheel.ps1"
NPM_WINDOWS="$SMOKE_DIR/smoke-npm-package.ps1"
PY_CONSUMER="$SMOKE_DIR/registry-python.py"
NODE_CONSUMER="$SMOKE_DIR/registry-node.mjs"

check_common "$CRATES" "smoke-crates-cli"
# Exact-version cargo install (--version "=$VERSION", not a compatible range).
assert_contains "smoke-crates-cli: exact cargo install" "$CRATES" \
  'cargo install fathomdb-cli --version "=$VERSION"'
# JSON parses post-run.
assert_contains "smoke-crates-cli: jq parses check-integrity output" "$CRATES" \
  'jq -e . >/dev/null'

check_common "$PYPI" "smoke-pypi-wheel"
assert_contains "smoke-pypi-wheel: fresh venv" "$PYPI" 'python3 -m venv'
assert_contains "smoke-pypi-wheel: pinned pip install" "$PYPI" \
  'pip install --quiet "fathomdb==${PIP_VERSION}"'
# PEP 440 normalization helper present (SemVer -rc.N -> PEP 440 rcN).
assert_contains "smoke-pypi-wheel: PEP 440 normalization" "$PYPI" \
  'pep440_normalize()'
assert_contains "smoke-pypi-wheel: open/close exercise" "$PY_CONSUMER" 'Engine.open'
assert_contains "smoke-pypi-wheel: close call" "$PY_CONSUMER" 'engine.close()'
# Canonical writes require provenance. Keep this structural assertion alongside
# the real-registry smoke contract so a future edit cannot reintroduce a
# publish-only WriteValidationError.
assert_contains "smoke-pypi-wheel: write carries source_id" "$PY_CONSUMER" \
  '"source_id": "smoke:pypi-wheel"'

check_common "$NPM" "smoke-npm-package"
assert_contains "smoke-npm-package: fresh npm init" "$NPM" 'npm init -y'
assert_contains "smoke-npm-package: pinned npm install" "$NPM" \
  'npm install --loglevel=warn "fathomdb@${VERSION}"'
assert_contains "smoke-npm-package: Engine.open exercise" "$NODE_CONSUMER" 'Engine.open'
assert_contains "smoke-npm-package: close call" "$NODE_CONSUMER" 'await engine.close()'
# TypeScript accepts camelCase at the N-API boundary; the opaque id is the
# smoke fixture's provenance, not caller content.
assert_contains "smoke-npm-package: write carries sourceId" "$NODE_CONSUMER" \
  'sourceId: "smoke:npm-package"'

# Windows uses PowerShell rather than assuming a Unix shell. Its scripts retain
# the same registry-pinned open/write/search/close contract as the bash legs.
assert_contains "windows PyPI smoke: creates a fresh venv" "$PYPI_WINDOWS" 'python -m venv'
assert_contains "windows PyPI smoke: pinned install" "$PYPI_WINDOWS" '"fathomdb==$Version"'
assert_contains "windows PyPI smoke: uses the shared wheel consumer" "$PYPI_WINDOWS" 'registry-python.py'
assert_contains "windows npm smoke: pinned install" "$NPM_WINDOWS" '"fathomdb@$Version"'
assert_contains "windows npm smoke: uses the shared Node consumer" "$NPM_WINDOWS" 'registry-node.mjs'

# `$ErrorActionPreference` does not reliably turn a nonzero native-process
# exit into a terminating PowerShell error.  Every native command therefore
# needs an adjacent `$LASTEXITCODE` guard, so a later successful command
# cannot mask an install or runtime failure.
native_exit_guard='  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }'
assert_next_line "windows PyPI smoke: venv propagates native failure" "$PYPI_WINDOWS" \
  '  python -m venv "$work/venv"' "$native_exit_guard"
assert_next_line "windows PyPI smoke: pip upgrade propagates native failure" "$PYPI_WINDOWS" \
  '  & $python -m pip install --quiet --upgrade pip' "$native_exit_guard"
assert_next_line "windows PyPI smoke: wheel install propagates native failure" "$PYPI_WINDOWS" \
  '  & $python -m pip install --quiet "fathomdb==$Version"' "$native_exit_guard"
assert_next_line "windows PyPI smoke: SDK exercise propagates native failure" "$PYPI_WINDOWS" \
  '  & $python -I $consumer $Version $db write' "$native_exit_guard"
assert_next_line "windows PyPI smoke: reopen propagates native failure" "$PYPI_WINDOWS" \
  '  & $python -I $consumer $Version $db reopen' "$native_exit_guard"
assert_next_line "windows npm smoke: npm init propagates native failure" "$NPM_WINDOWS" \
  '  npm init -y | Out-Null' "$native_exit_guard"
assert_next_line "windows npm smoke: npm install propagates native failure" "$NPM_WINDOWS" \
  '  npm install --loglevel=warn "fathomdb@$Version" *> $installLog' '  $installExit = $LASTEXITCODE'
assert_contains "windows npm smoke: original install exit takes precedence" "$NPM_WINDOWS" \
  'if ($installExit -ne 0) { exit $installExit }'
assert_next_line "windows npm smoke: Node exercise propagates native failure" "$NPM_WINDOWS" \
  '  node $consumer write $work $Version' "$native_exit_guard"
assert_next_line "windows npm smoke: reopen propagates native failure" "$NPM_WINDOWS" \
  '  node $consumer reopen $work $Version' "$native_exit_guard"

# Run each smoke with a bad version arg — must exit non-zero BEFORE doing
# any network work, with a usage-shaped diagnostic.
for s in "$CRATES" "$PYPI" "$NPM"; do
  name="$(basename "$s")"
  if out="$("$s" not-a-semver 2>&1)"; then
    fail "$name: non-semver version should be rejected"
  else
    if printf '%s' "$out" | grep -qiE 'invalid|usage|version'; then
      pass "$name: non-semver rejected pre-install"
    else
      fail "$name: wrong diagnostic for non-semver; got: $out"
    fi
  fi
  if out="$("$s" 2>&1)"; then
    fail "$name: missing arg should fail"
  else
    if printf '%s' "$out" | grep -qi usage; then
      pass "$name: missing arg → usage"
    else
      fail "$name: wrong diagnostic for missing arg; got: $out"
    fi
  fi
done

SDK="$SMOKE_DIR/smoke-crates-sdk.sh"
check_common "$SDK" "smoke-crates-sdk"
assert_contains "Rust SDK reads in a separate process" "$SDK" '"$consumer" "$work/smoke.sqlite" reopen'
assert_contains "Python SDK reads in a separate process" "$PYPI" '"$consumer" "$PIP_VERSION" "$DB" reopen'
assert_contains "Windows Python SDK reads in a separate process" "$PYPI_WINDOWS" '& $python -I $consumer $Version $db reopen'
assert_contains "Node SDK reads in a separate process" "$NPM" 'node "$consumer" reopen "$WORK" "$VERSION"'
assert_contains "Windows Node SDK reads in a separate process" "$NPM_WINDOWS" 'node $consumer reopen $work $Version'

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin"
cat > "$fixture/bin/npm" <<'SHIM'
#!/usr/bin/env bash
set -euo pipefail
if [ "$1" = init ]; then
  printf '{}\n' > package.json
  exit 0
fi
[ "$1" = install ]
printf 'npm fixture: optional dependency failed to resolve\n'
printf 'NPM_TOKEN=synthetic-private-marker\n'
[ "${SMOKE_INSTALL_EXIT:-0}" = 0 ] || exit "$SMOKE_INSTALL_EXIT"
mkdir -p node_modules/fathomdb node_modules/fathomdb-linux-x64-gnu node_modules/fathomdb-linux-arm64-gnu
printf '{"name":"fathomdb","version":"0.8.30","optionalDependencies":{"fathomdb-linux-x64-gnu":"0.8.30"}}\n' > node_modules/fathomdb/package.json
printf '{"name":"fathomdb-linux-x64-gnu","version":"0.8.30","main":"fathomdb.linux-x64-gnu.node"}\n' > node_modules/fathomdb-linux-x64-gnu/package.json
printf 'wrong platform binary\n' > node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node
SHIM
chmod +x "$fixture/bin/npm"
host_machine="$(uname -sm)"
if [ "$host_machine" = "Linux x86_64" ] && node -e 'process.exit(process.report.getReport().header.glibcVersionRuntime ? 0 : 1)'; then
  result=0
  diagnostic="$(PATH="$fixture/bin:$PATH" bash "$NPM" 0.8.30 2>&1)" || result=$?
  if [ "$result" -ne 0 ] && [[ "$diagnostic" == *"requested=0.8.30 resolved=0.8.30"* ]] \
    && [[ "$diagnostic" == *"optional=fathomdb-linux-x64-gnu expected_version=0.8.30 installed_version=0.8.30"* ]] \
    && [[ "$diagnostic" == *"expected_path="*"/fathomdb.linux-x64-gnu.node"* ]] \
    && [[ "$diagnostic" == *"observed_native=missing"* ]] \
    && [[ "$diagnostic" == *"optional dependency failed to resolve"* ]] \
    && [[ "$diagnostic" != *synthetic-private-marker* ]]; then
    pass "Bash npm smoke rejects a missing matching addon even with a wrong-platform binary installed"
  else
    fail "npm missing-native diagnostic lost version, path, install output or redaction: exit=$result output=$diagnostic"
  fi
  result=0
  diagnostic="$(PATH="$fixture/bin:$PATH" SMOKE_INSTALL_EXIT=43 bash "$NPM" 0.8.30 2>&1)" || result=$?
  if [ "$result" -eq 43 ] && [[ "$diagnostic" == *"optional dependency failed to resolve"* ]] \
    && [[ "$diagnostic" != *synthetic-private-marker* ]]; then
    pass "Bash npm smoke preserves the exact install failure and redacted output"
  else
    fail "npm install failure lost its exit, diagnostic or redaction: exit=$result output=$diagnostic"
  fi
fi

if [ "$FAILED" -gt 0 ]; then
  printf '\n%d test(s) failed\n' "$FAILED" >&2
  exit 1
fi
printf '\nAll smoke-script structural tests passed\n'
