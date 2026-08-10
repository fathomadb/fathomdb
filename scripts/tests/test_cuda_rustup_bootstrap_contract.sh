#!/usr/bin/env bash
# Mutation fixtures for the pinned CUDA Rust bootstrap contract.
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

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
text += '''
ARG SECOND_BOOTSTRAP_URL
RUN curl --proto '=https' --tlsv1.2 --fail --silent --show-error --output /tmp/second-bootstrap "$SECOND_BOOTSTRAP_URL"
'''
path.write_text(text)
PY
expect_fail "$FIXTURE" 'rejects an additional unverified Rust bootstrap download'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'ARG RUSTUP_INIT_URL=https://static.rust-lang.org/rustup/archive/1.29.0/x86_64-unknown-linux-gnu/rustup-init'
if text.count(needle) != 1:
    raise SystemExit('fixture no longer contains exactly one fixed rustup-init URL')
path.write_text(text.replace(needle, 'ARG RUSTUP_INIT_URL', 1))
PY
expect_fail "$FIXTURE" 'requires the fixed rustup-init artifact URL'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'ARG RUSTUP_INIT_SHA256=4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10'
if text.count(needle) != 1:
    raise SystemExit('fixture no longer contains exactly one rustup-init checksum')
path.write_text(text.replace(needle, 'ARG RUSTUP_INIT_SHA256', 1))
PY
expect_fail "$FIXTURE" 'requires the fixed rustup-init checksum'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'sha256sum --check --status'
if text.count(needle) != 1:
    raise SystemExit('fixture no longer contains exactly one checksum verification command')
path.write_text(text.replace(needle, 'sha256sum --check', 1))
PY
expect_fail "$FIXTURE" 'requires a checked rustup-init checksum verification'

printf '\nCUDA rustup-bootstrap-contract tests passed\n'
