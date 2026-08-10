#!/usr/bin/env bash
# Regression fixtures for the image-owned auditwheel report boundary.
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
  cp "$REPO_ROOT/scripts/release/cuda-image-attestation.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/build-napi-cuda.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-preflight.sh" "$root/scripts/release/"
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

if grep -Fq 'auditwheel show "$WHEEL"' "$REPO_ROOT/scripts/release/cuda-preflight.sh"; then
  printf 'FAIL  auditwheel must run in the attested CUDA manylinux image, never on the host\n' >&2
  exit 1
fi

FIXTURE="$TMPROOT/fixture"
make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'auditwheel show "/witness/python-dist/$WHEEL_BASENAME"'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer runs auditwheel exactly once inside the mounted image witness")
path.write_text(text.replace(needle, 'auditwheel show "$WHEEL"', 1))
PY
expect_fail "$FIXTURE" 'rejects a host-side auditwheel report'

printf '\nCUDA auditwheel-boundary tests passed\n'
