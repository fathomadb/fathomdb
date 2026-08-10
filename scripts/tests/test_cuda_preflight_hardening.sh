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

if grep -Fq -- '--mount "type=bind,src=$CUDA_TOOLKIT_ROOT,dst=/opt/cuda,readonly"' \
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
needle = "      io.fathomdb.cuda.rust=$RUST_VERSION \\\n+"
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
needle = "--query-compute-apps=pid"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one process-bound CUDA witness query")
path.write_text(text.replace(needle, "--query-gpu=name", 1))
PY
expect_fail "$FIXTURE" 'rejects GPU smokes without a process-bound CUDA witness'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "ldd \"$NAPI_BINARY\" || true"
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains the pre-hardening permissive Node ldd")
path.write_text(text.replace(needle, "ldd \"$NAPI_BINARY\"", 1))
PY
expect_fail "$FIXTURE" 'rejects dynamic dependency checks that tolerate ldd failure'

printf '\nCUDA preflight-hardening tests passed\n'
