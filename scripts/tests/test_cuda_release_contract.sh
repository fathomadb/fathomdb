#!/usr/bin/env bash
# Regression fixtures for the Linux CUDA release-contract checker.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="${CHECKER_UNDER_TEST:-$REPO_ROOT/scripts/check-cuda-release-contract.py}"

TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

make_fixture() {
  local root="$1"
  mkdir -p "$root/.github/workflows" "$root/scripts/release" "$root/src/rust/crates/fathomdb-napi" "$root/src/ts"
  cp "$REPO_ROOT/.github/workflows/release.yml" "$root/.github/workflows/"
  cp "$REPO_ROOT/scripts/release/cuda-artifact-contract.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/build-napi-cuda.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-preflight.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/src/rust/crates/fathomdb-napi/Cargo.toml" "$root/src/rust/crates/fathomdb-napi/"
  cp "$REPO_ROOT/src/ts/package.json" "$root/src/ts/"
}

expect_pass() {
  local root="$1" description="$2"
  if REPO_ROOT="$root" python3 "$CHECKER" >/dev/null; then
    printf 'PASS  %s\n' "$description"
  else
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
}

expect_fail() {
  local root="$1" description="$2"
  if REPO_ROOT="$root" python3 "$CHECKER" >/dev/null 2>&1; then
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
  printf 'PASS  %s\n' "$description"
}

FIXTURE="$TMPROOT/fixture"
make_fixture "$FIXTURE"
expect_pass "$FIXTURE" 'baseline CUDA contract agrees'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/src/rust/crates/fathomdb-napi/Cargo.toml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'embed-cuda = ["default-embedder", "fathomdb-engine/embed-cuda"]\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one N-API CUDA forwarding feature")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a N-API binary without CUDA feature forwarding'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '    runs-on: [self-hosted, Linux, X64, gpu, cuda-12]\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains the restricted CUDA runner selection")
path.write_text(text.replace(needle, '    runs-on: ubuntu-latest\n', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA preflight moved onto ordinary CI'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "    if: ${{ github.event_name == 'workflow_dispatch' && inputs.dry_run == true }}\n"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one dry-run-only CUDA guard")
path.write_text(text.replace(needle, "    if: ${{ github.event_name == 'workflow_dispatch' }}\n", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA preflight that could run on a publishing dispatch'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '--network none'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one network-isolated driverless smoke")
path.write_text(text.replace(needle, '--network host', 1))
PY
expect_fail "$FIXTURE" 'rejects a driverless smoke without network isolation'

printf '\nCUDA release-contract tests passed\n'
