#!/usr/bin/env bash
# RED/GREEN fixtures for the image-owned CUDA build and GPU smoke hardening.
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
  cp "$REPO_ROOT/scripts/release/cuda-preflight-witness.schema.json" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/verify-cuda-preflight-witness.py" "$root/scripts/release/"
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

if grep -Fq -- "--mount \"type=bind,src=\$CUDA_TOOLKIT_ROOT,dst=/opt/cuda,readonly\"" \
  "$REPO_ROOT/scripts/release/cuda-preflight.sh"; then
  printf 'FAIL  Python CUDA wheel build must use only the provisioned image toolkit\n' >&2
  exit 1
fi

FIXTURE="$TMPROOT/fixture"

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "io.fathomdb.cuda.rust=$RUST_VERSION"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one Rust provenance label")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a provisioned image without the pinned Rust label'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "sha256sum --check --status"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one rustup-init checksum verification")
path.write_text(text.replace(needle, "true", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA image whose rustup-init is unchecked'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "--query-compute-apps=pid,process_name --format=csv,noheader"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains the shared process-bound CUDA witness query")
path.write_text(text.replace(needle, "--query-gpu=name", 1))
PY
expect_fail "$FIXTURE" 'rejects GPU smokes without a process-bound CUDA witness'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "printf '\\n[node ldd]\\n'; ldd \"$NAPI_BINARY\""
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one strict Node ldd")
path.write_text(text.replace(needle, needle + " || true", 1))
PY
expect_fail "$FIXTURE" 'rejects dynamic dependency checks that tolerate ldd failure'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '--user "$CONTAINER_USER"'
if text.count(needle) != 6:
    raise SystemExit("fixture must contain exactly six non-root writable CUDA containers")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a writable CUDA container that runs as Docker root'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '--mount "type=bind,src=$REPO_ROOT,dst=/workspace,readonly"'
if text.count(needle) != 1:
    raise SystemExit("fixture must contain exactly one read-only CUDA workspace mount")
path.write_text(text.replace(needle, '--mount "type=bind,src=$REPO_ROOT,dst=/workspace"', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA wheel build with a writable checkout mount'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '-e CARGO_TARGET_DIR=/tmp/fathomdb-cargo-target'
if text.count(needle) != 1:
    raise SystemExit("fixture must contain exactly one container-local Cargo target directory")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA wheel build without a container-local Cargo target directory'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-artifact-contract.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "CUDA_RUSTUP_TOOLCHAIN='1.95.0-x86_64-unknown-linux-gnu'"
if text.count(needle) != 1:
    raise SystemExit("fixture must contain exactly one pinned non-root Rustup toolchain")
path.write_text(text.replace(needle, "CUDA_RUSTUP_TOOLCHAIN='stable'", 1))
PY
expect_fail "$FIXTURE" 'rejects an unpinned non-root Rustup toolchain'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '-e "RUSTUP_TOOLCHAIN=$CUDA_RUSTUP_TOOLCHAIN"'
if text.count(needle) != 1:
    raise SystemExit("fixture must contain exactly one non-root Rustup toolchain environment")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA wheel build without the pinned non-root Rustup toolchain'

printf '\nCUDA preflight-hardening tests passed\n'
