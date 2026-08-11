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
  cp "$REPO_ROOT/Cargo.toml" "$REPO_ROOT/Cargo.lock" "$root/"
  cp "$REPO_ROOT/.github/workflows/release.yml" "$root/.github/workflows/"
  cp "$REPO_ROOT/scripts/release/cuda-artifact-contract.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/build-napi-cuda.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-preflight.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-image-attestation.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/Dockerfile.cuda-manylinux" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/provision-cuda-manylinux.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/src/rust/crates/fathomdb-napi/Cargo.toml" "$root/src/rust/crates/fathomdb-napi/"
  cp "$REPO_ROOT/src/ts/package.json" "$root/src/ts/"
}

require_provisioning_assets() {
  local file
  for file in \
    "$REPO_ROOT/scripts/release/Dockerfile.cuda-manylinux" \
    "$REPO_ROOT/scripts/release/provision-cuda-manylinux.sh"; do
    if [ ! -f "$file" ]; then
      printf 'FAIL  missing reproducible CUDA provisioning asset: %s\n' "$file" >&2
      exit 1
    fi
  done
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

require_provisioning_assets

FIXTURE="$TMPROOT/fixture"
make_fixture "$FIXTURE"
expect_pass "$FIXTURE" 'baseline CUDA contract agrees'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '-e "LIBRARY_PATH=$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB"'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one CUDA runtime linker search path")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA build without image-owned runtime link search paths'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/build-napi-cuda.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'export LIBRARY_PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64${LIBRARY_PATH:+:$LIBRARY_PATH}"\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one N-API CUDA runtime linker search path")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects an N-API CUDA build without a toolkit runtime link search path'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/Cargo.toml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'candle-nn-fathomdb = { git = "https://github.com/coreyt/candle-fathomdb.git", rev = "5719d90e60edd14c4c1a3bf87952648131b2153a" }\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one Candle NN source pin")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a split Candle source selection'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/Cargo.lock" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'source = "git+https://github.com/coreyt/candle-fathomdb.git?rev=5719d90e60edd14c4c1a3bf87952648131b2153a#5719d90e60edd14c4c1a3bf87952648131b2153a"'
if text.count(needle) != 3:
    raise SystemExit("fixture no longer contains all three immutable Candle lock sources")
path.write_text(text.replace(needle, 'source = "registry+https://github.com/rust-lang/crates.io-index"', 1))
PY
expect_fail "$FIXTURE" 'rejects a Candle lockfile that falls back to crates.io'

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
if text.count(needle) < 2:
    raise SystemExit("fixture no longer contains both network-isolated driverless smokes")
path.write_text(text.replace(needle, '--network host', 1))
PY
expect_fail "$FIXTURE" 'rejects either driverless smoke without network isolation'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'Engine.open(str(db_path), use_default_embedder=True)'
if needle not in text:
    needle = 'Engine.open(str(db_path))'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one driverless Python open")
path.write_text(text.replace(needle, 'Engine.open(str(db_path), use_default_embedder=False)', 1))
PY
expect_fail "$FIXTURE" 'rejects a driverless Python smoke that skips the default embedder'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '{ useDefaultEmbedder: true }'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one driverless N-API default-embedder open")
path.write_text(text.replace(needle, '{ useDefaultEmbedder: false }', 1))
PY
expect_fail "$FIXTURE" 'rejects a driverless installed N-API smoke that skips the default embedder'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '--mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly"'
if text.count(needle) < 2:
    raise SystemExit("fixture no longer mounts the local default-embedder mirror for both smokes")
path.write_text(text.replace(needle, '--mount "type=bind,src=/missing-model-cache,dst=/fathomdb-hf,readonly"', 1))
PY
expect_fail "$FIXTURE" 'rejects a driverless artifact smoke without the pinned local model mirror'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '    runs-on: [self-hosted, Linux, X64, gpu, cuda-12]\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one CUDA runner selection")
path.write_text(text.replace(needle, needle + '    permissions:\n      contents: write\n', 1))
PY
expect_fail "$FIXTURE" 'rejects CUDA preflight permissions broader than read-only'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a'
if text.count(needle) != 3:
    raise SystemExit("fixture CUDA witness upload must use the reviewed full artifact SHA")
path.write_text(text.replace(needle, needle[:-1], 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA witness uploader with a shortened action SHA'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "@sha256:"
if needle not in text:
    raise SystemExit("fixture Dockerfile no longer pins its base image by digest")
path.write_text(text.replace(needle, ":", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA manylinux Dockerfile with a mutable base image'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/provision-cuda-manylinux.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'sha256sum --check --status'
if text.count(needle) != 1:
    raise SystemExit("fixture provisioner no longer verifies exactly one pinned cache manifest")
path.write_text(text.replace(needle, 'true', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA provisioner that accepts an unchecked model cache'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'sha256sum --check --status'
if text.count(needle) != 1:
    raise SystemExit("fixture preflight no longer verifies exactly one pinned cache manifest")
path.write_text(text.replace(needle, 'true', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA preflight that accepts an unchecked model cache'

printf '\nCUDA release-contract tests passed\n'
