#!/usr/bin/env bash
# Fail closed unless the local manylinux image was built from this contract.

assert_cuda_manylinux_image() {
  local label value actual_label
  if ! docker image inspect "$CUDA_MANYLINUX_IMAGE" >/dev/null 2>&1; then
    printf 'cuda image attestation: required image is absent: %s\n' "$CUDA_MANYLINUX_IMAGE" >&2
    return 1
  fi

  while IFS='=' read -r label value; do
    if ! actual_label="$(docker image inspect --format "{{ index .Config.Labels \"$label\" }}" "$CUDA_MANYLINUX_IMAGE")"; then
      printf 'cuda image attestation: cannot inspect required image %s\n' "$CUDA_MANYLINUX_IMAGE" >&2
      return 1
    fi
    if [ "$actual_label" != "$value" ]; then
      printf 'cuda image attestation: image %s has %s=%s; expected %s\n' \
        "$CUDA_MANYLINUX_IMAGE" "$label" "$actual_label" "$value" >&2
      return 1
    fi
  done <<EOF
io.fathomdb.cuda.manylinux-base=$CUDA_MANYLINUX_BASE_IMAGE
io.fathomdb.cuda.toolkit-base=$CUDA_TOOLKIT_IMAGE
io.fathomdb.cuda.toolkit=$CUDA_TOOLKIT_VERSION
io.fathomdb.cuda.manylinux=$CUDA_MANYLINUX
io.fathomdb.cuda.python=$CUDA_MANYLINUX_PYTHON_ABI
io.fathomdb.cuda.rust=$CUDA_RUST_VERSION
io.fathomdb.cuda.maturin=$CUDA_MATURIN_VERSION
io.fathomdb.cuda.compiler=$CUDA_MANYLINUX_GCC_TOOLSET
io.fathomdb.cuda.compiler-version=$CUDA_MANYLINUX_GCC_VERSION
io.fathomdb.cuda.rustup-init-sha256=$CUDA_RUSTUP_INIT_SHA256
EOF
}
