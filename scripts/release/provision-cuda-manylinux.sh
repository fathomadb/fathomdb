#!/usr/bin/env bash
# Build the exact CUDA/manylinux image and warm only the default-embedder cache
# required by cuda-preflight.sh. Run this only on the designated release runner.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"

DEFAULT_EMBEDDER_HF_HOME="${FATHOMDB_CUDA_PREFLIGHT_HF_HOME:-${HF_HOME:-$HOME/.cache/huggingface}}"
DEFAULT_EMBEDDER_SNAPSHOT="$DEFAULT_EMBEDDER_HF_HOME/hub/models--${CUDA_DEFAULT_EMBEDDER_HF_REPO//\//--}/snapshots/$CUDA_DEFAULT_EMBEDDER_HF_REVISION"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'provision-cuda-manylinux: required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

default_embedder_snapshot_is_verified() {
  {
    printf '%s  %s/config.json\n' "$CUDA_DEFAULT_EMBEDDER_CONFIG_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
    printf '%s  %s/tokenizer.json\n' "$CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
    printf '%s  %s/model.safetensors\n' "$CUDA_DEFAULT_EMBEDDER_MODEL_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
  } | sha256sum --check --status
}

download_file() {
  local file_name="$1" destination temporary url
  destination="$DEFAULT_EMBEDDER_SNAPSHOT/$file_name"
  url="https://huggingface.co/${CUDA_DEFAULT_EMBEDDER_HF_REPO}/resolve/${CUDA_DEFAULT_EMBEDDER_HF_REVISION}/${file_name}"
  temporary="$(mktemp "$DEFAULT_EMBEDDER_SNAPSHOT/.${file_name}.XXXXXX")"
  if ! curl --fail --location --retry 3 --retry-all-errors --output "$temporary" "$url"; then
    rm -f "$temporary"
    exit 1
  fi
  chmod 0644 "$temporary"
  mv -f "$temporary" "$destination"
}

validate_image() {
  local label value actual_label
  while IFS='=' read -r label value; do
    if ! actual_label="$(docker image inspect --format "{{ index .Config.Labels \"$label\" }}" "$CUDA_MANYLINUX_IMAGE")"; then
      printf 'provision-cuda-manylinux: cannot inspect required image %s\n' "$CUDA_MANYLINUX_IMAGE" >&2
      exit 1
    fi
    if [ "$actual_label" != "$value" ]; then
      printf 'provision-cuda-manylinux: image %s lacks expected %s=%s\n' \
        "$CUDA_MANYLINUX_IMAGE" "$label" "$value" >&2
      exit 1
    fi
  done <<EOF
io.fathomdb.cuda.toolkit=$CUDA_TOOLKIT_VERSION
io.fathomdb.cuda.manylinux=$CUDA_MANYLINUX
io.fathomdb.cuda.python=$CUDA_MANYLINUX_PYTHON_ABI
io.fathomdb.cuda.rust=$CUDA_RUST_VERSION
io.fathomdb.cuda.maturin=$CUDA_MATURIN_VERSION
EOF

  docker run --rm --network none --platform "$CUDA_MANYLINUX_PLATFORM" \
    -e CUDA_RUST_VERSION -e CUDA_MATURIN_VERSION \
    "$CUDA_MANYLINUX_IMAGE" sh -ceu '
    test -x /opt/python/cp311-cp311/bin/python
    /opt/python/cp311-cp311/bin/python --version | grep -E "Python 3\.11\."
    rustc --version | grep -F "rustc $CUDA_RUST_VERSION"
    maturin --version | grep -F "maturin $CUDA_MATURIN_VERSION"
    /usr/local/cuda-12.6/bin/nvcc --version | grep -F "release 12.6"
  '
}

for command in curl docker sha256sum; do
  require_command "$command"
done
if ! docker info >/dev/null 2>&1; then
  printf 'provision-cuda-manylinux: Docker must be available on the designated runner\n' >&2
  exit 1
fi

mkdir -p "$DEFAULT_EMBEDDER_SNAPSHOT"
if ! default_embedder_snapshot_is_verified; then
  download_file config.json
  download_file tokenizer.json
  download_file model.safetensors
  if ! default_embedder_snapshot_is_verified; then
    printf 'provision-cuda-manylinux: incomplete or unverified default-embedder cache at %s\n' \
      "$DEFAULT_EMBEDDER_SNAPSHOT" >&2
    exit 1
  fi
fi

docker build --platform "$CUDA_MANYLINUX_PLATFORM" \
  --tag "$CUDA_MANYLINUX_IMAGE" \
  --build-arg "RUST_VERSION=$CUDA_RUST_VERSION" \
  --build-arg "MATURIN_VERSION=$CUDA_MATURIN_VERSION" \
  --file "$REPO_ROOT/$CUDA_MANYLINUX_DOCKERFILE" \
  "$REPO_ROOT/scripts/release"
validate_image

printf 'provision-cuda-manylinux: pass; image=%s cache=%s\n' \
  "$CUDA_MANYLINUX_IMAGE" "$DEFAULT_EMBEDDER_SNAPSHOT"
