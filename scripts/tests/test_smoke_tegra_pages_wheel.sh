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
if grep -qE "printf.*\\|[[:space:]]*grep[[:space:]]+-q" "$SMOKE"; then
  fail "does not pipe input validation through an early-exiting grep"
else
  pass "does not pipe input validation through an early-exiting grep"
fi

for args in \
  '0.8.24 https://fathomadb.github.io/fathomdb/tegra/simple/ 0123456789012345678901234567890123456789012345678901234567890123 2431f8729afb247518804e90b9ca324592c95456 32878233246 /tmp/tegra-smoke-invalid' \
  '0.8.24+tegra http://example.invalid/simple/ 0123456789012345678901234567890123456789012345678901234567890123 2431f8729afb247518804e90b9ca324592c95456 32878233246 /tmp/tegra-smoke-invalid' \
  '0.8.24+tegra https://fathomadb.github.io/fathomdb/tegra/simple/ not-a-digest 2431f8729afb247518804e90b9ca324592c95456 32878233246 /tmp/tegra-smoke-invalid' \
  '0.8.24+tegra https://fathomadb.github.io/fathomdb/tegra/simple/ 0123456789012345678901234567890123456789012345678901234567890123 2431f8729afb247518804e90b9ca324592c95456 32878233246 /tmp/tegra-smoke-invalid --extra-index-url https://example.invalid/simple/'; do
  read -r -a argv <<<"$args"
  if output="$("$SMOKE" "${argv[@]}" 2>&1)"; then
    fail "invalid input fails before download: $args"
  else
    case "${output,,}" in
      *invalid* | *usage* | *expected* | *must*)
        pass "invalid input fails before download: $args"
        ;;
      *) fail "invalid input has actionable diagnostic: $args" ;;
    esac
  fi
done

if [ "$FAILED" -ne 0 ]; then
  printf '%s Tegra Pages smoke structural test(s) failed\n' "$FAILED" >&2
  exit 1
fi

# A selected absent local wheel must fail as a missing artifact, before any
# platform/GPU probe; it must not be reinterpreted as a Pages version or URL.
set +e
local_output="$(bash "$SMOKE" --local-wheel /tmp/slice130-absent.whl 0.8.29+tegra \
  0123456789012345678901234567890123456789012345678901234567890123 \
  0123456789012345678901234567890123456789 /tmp/slice130-unused 2>&1)"
local_status=$?
set -e
if [ "$local_status" -ne 1 ] || ! grep -Fq 'local wheel is absent' <<<"$local_output"; then
  printf 'FAIL local wheel source was not handled before platform probes: exit=%s\n%s\n' "$local_status" "$local_output" >&2
  exit 1
fi

# Run the actual per-policy shell command with a native-free child. A CPU
# control cannot request a GPU allocation witness, including via inherited env.
python3 - "$SMOKE" <<'PY_POLICY_CONTROL'
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

source = Path(sys.argv[1]).read_text()
preamble = source[source.index("export FATHOMDB_EMBED_DEVICE=cuda:0"):source.index('GPU_INFO=')]
start = source.index("for policy in cpu auto cuda:0; do")
body = source.index("<<'PY'\n", start) + len("<<'PY'\n")
command = source[start:body] + '''import json
import os
print(json.dumps([os.environ.get(key) for key in (
    "FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE", "FATHOMDB_GPU_ALLOCATION_WITNESS")]))
PY
done
'''
with tempfile.TemporaryDirectory() as directory:
    result = subprocess.run(["bash", "-ec", preamble + command],
                            env=dict(os.environ, WORK=directory, FATHOMDB_GPU_ALLOCATION_WITNESS="1"),
                            capture_output=True, text=True, timeout=10)
assert result.returncode == 0, result.stderr
rows = [json.loads(line) for line in result.stdout.splitlines()]
assert rows == [["cpu", "cpu", None], ["auto", "auto", "1"], ["cuda:0", "cuda:0", "1"]], rows
print("PASS  shipping CPU clears inherited witness request; auto/CUDA require witnesses")
PY_POLICY_CONTROL
