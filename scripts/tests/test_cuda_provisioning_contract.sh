#!/usr/bin/env bash
# Mutation fixtures for the designated-runner CUDA provisioning contract.
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
  cp "$REPO_ROOT/scripts/release/cuda-image-attestation.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/Dockerfile.cuda-manylinux" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/provision-cuda-manylinux.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/src/rust/crates/fathomdb-napi/Cargo.toml" "$root/src/rust/crates/fathomdb-napi/"
  cp "$REPO_ROOT/src/ts/package.json" "$root/src/ts/"
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

if ! grep -Fq "NVCC_CCBIN=/opt/rh/gcc-toolset-13/root/usr/bin/g++" \
  "$REPO_ROOT/scripts/release/Dockerfile.cuda-manylinux"; then
  printf 'FAIL  CUDA manylinux image must pin nvcc to the GCC 13 host compiler\n' >&2
  exit 1
fi

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-artifact-contract.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "CUDA_MANYLINUX_IMAGE='fathomdb-cuda-manylinux:12.6-manylinux_2_28'"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one CUDA manylinux image tag")
path.write_text(text.replace(needle, "CUDA_MANYLINUX_IMAGE='fathomdb-cuda-manylinux:latest'", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA provisioner image tag that is not the release tag'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-artifact-contract.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "CUDA_RUST_VERSION='1.95.0'"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains the pinned Rust toolchain")
path.write_text(text.replace(needle, "CUDA_RUST_VERSION='stable'", 1))
PY
expect_fail "$FIXTURE" 'rejects an unpinned CUDA builder Rust toolchain'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "CUDACXX=/usr/local/cuda-12.6/bin/nvcc"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains the image-owned CUDA compiler selection")
path.write_text(text.replace(needle, "CUDACXX=/usr/bin/g++", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA builder without the pinned GCC 13 compiler selection'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "CUDAHOSTCXX=/opt/rh/gcc-toolset-13/root/usr/bin/g++"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one pinned CUDA host compiler")
path.write_text(text.replace(needle, "CUDAHOSTCXX=/usr/bin/g++", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA builder whose nvcc host compiler is not GCC 13'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-artifact-contract.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "CUDA_DEFAULT_EMBEDDER_MODEL_SHA256='3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad'"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains the pinned model digest")
path.write_text(text.replace(needle, "CUDA_DEFAULT_EMBEDDER_MODEL_SHA256='0000000000000000000000000000000000000000000000000000000000000000'", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA provisioner cache digest that is not the pinned model digest'

printf '\nCUDA provisioning-contract tests passed\n'
