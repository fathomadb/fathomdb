#!/usr/bin/env bash
# Structural contract for the public, exact-version Tegra Pages smoke.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SMOKE="$REPO_ROOT/scripts/release/smoke/smoke-tegra-pages-wheel.sh"
FAILED=0

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }
contains() {
  local label="$1" needle="$2"
  if grep -qF -- "$needle" "$SMOKE"; then pass "$label"; else fail "$label"; fi
}

if [ ! -x "$SMOKE" ]; then
  fail "Tegra Pages smoke exists and is executable"
  exit 1
fi
pass "Tegra Pages smoke exists and is executable"

contains "hardens bash" 'set -euo pipefail'
contains "requires a durable evidence directory" '<new-evidence-dir>'
contains "uses the single supplied index" "pip_clean download --quiet --index-url \"\$INDEX_URL\""
contains "ignores pip configuration files" 'PIP_CONFIG_FILE=/dev/null'
contains "clears pip extra-index injection" '-u PIP_EXTRA_INDEX_URL'
contains "clears pip find-links injection" '-u PIP_FIND_LINKS'
contains "requires only binary wheels" '--only-binary=:all:'
contains "disables cache" '--no-cache-dir'
contains "checks the independently supplied SHA-256" "sha256sum \"\$WHEEL\""
contains "installs only the checked download" "pip_clean install --quiet --no-index --no-deps \"\$WHEEL\""
contains "does not use an extra index" 'extra-index-url'
contains "opens the installed package" 'Engine.open'
contains "writes a provenanced record" '"source_id": "smoke:tegra-pages-wheel"'
contains "closes the engine" 'engine.close()'
contains "requires forced CUDA" 'FATHOMDB_EMBED_DEVICE=cuda:0'
contains "requires an allocation witness" 'FATHOMDB_GPU_ALLOCATION_WITNESS=1'
contains "validates the witness" 'verify-tegra-gpu-witness.py'
contains "requires a classic Tegra nvgpu confirmation" '*nvgpu*'

for args in \
  '0.8.24 https://fathomadb.github.io/fathomdb/tegra/simple/ 0123456789012345678901234567890123456789012345678901234567890123 2431f8729afb247518804e90b9ca324592c95456 32878233246 /tmp/tegra-smoke-invalid' \
  '0.8.24+tegra http://example.invalid/simple/ 0123456789012345678901234567890123456789012345678901234567890123 2431f8729afb247518804e90b9ca324592c95456 32878233246 /tmp/tegra-smoke-invalid' \
  '0.8.24+tegra https://fathomadb.github.io/fathomdb/tegra/simple/ not-a-digest 2431f8729afb247518804e90b9ca324592c95456 32878233246 /tmp/tegra-smoke-invalid' \
  '0.8.24+tegra https://fathomadb.github.io/fathomdb/tegra/simple/ 0123456789012345678901234567890123456789012345678901234567890123 2431f8729afb247518804e90b9ca324592c95456 32878233246 /tmp/tegra-smoke-invalid --extra-index-url https://example.invalid/simple/'; do
  read -r -a argv <<<"$args"
  if output="$("$SMOKE" "${argv[@]}" 2>&1)"; then
    fail "invalid input fails before download: $args"
  elif printf '%s' "$output" | grep -qiE 'invalid|usage|expected|must'; then
    pass "invalid input fails before download: $args"
  else
    fail "invalid input has actionable diagnostic: $args"
  fi
done

if [ "$FAILED" -ne 0 ]; then
  printf '%s Tegra Pages smoke structural test(s) failed\n' "$FAILED" >&2
  exit 1
fi
