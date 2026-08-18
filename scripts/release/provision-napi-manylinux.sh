#!/usr/bin/env bash
# Build the exact napi/manylinux image build-napi's linux-arm64-gnu row
# runs inside (Slice 80.1, AC80-1). Run this on any runner that will build
# that row — self-hosted or GitHub-hosted with Docker available.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=napi-artifact-contract.sh
. "$SCRIPT_DIR/napi-artifact-contract.sh"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'provision-napi-manylinux: required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

validate_image() {
  docker run --rm --network none --platform "$NAPI_MANYLINUX_PLATFORM" \
    -e NAPI_RUST_VERSION -e NAPI_NODE_VERSION \
    "$NAPI_MANYLINUX_IMAGE" sh -ceu '
    node --version | grep -F "v$NAPI_NODE_VERSION"
    rustc --version | grep -F "rustc $NAPI_RUST_VERSION"
    cargo --version
  '
}

for command in curl docker sha256sum; do
  require_command "$command"
done
if ! docker info >/dev/null 2>&1; then
  printf 'provision-napi-manylinux: Docker must be available on this runner\n' >&2
  exit 1
fi

docker build --platform "$NAPI_MANYLINUX_PLATFORM" \
  --tag "$NAPI_MANYLINUX_IMAGE" \
  --build-arg "RUST_VERSION=$NAPI_RUST_VERSION" \
  --build-arg "NODE_VERSION=$NAPI_NODE_VERSION" \
  --build-arg "NODE_SHA256=$NAPI_NODE_LINUX_ARM64_SHA256" \
  --build-arg "RUSTUP_INIT_URL=$NAPI_RUSTUP_INIT_URL" \
  --build-arg "RUSTUP_INIT_SHA256=$NAPI_RUSTUP_INIT_SHA256" \
  --file "$REPO_ROOT/$NAPI_MANYLINUX_DOCKERFILE" \
  "$REPO_ROOT/scripts/release"
validate_image

printf 'provision-napi-manylinux: pass; image=%s\n' "$NAPI_MANYLINUX_IMAGE"
