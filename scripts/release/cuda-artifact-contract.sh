#!/usr/bin/env bash
# 0.8.23's one checked source for the Linux CUDA artifact build inputs.
#
# It is sourced only by release preflight/build helpers. CPU CI and every
# non-Linux release lane deliberately retain their existing feature lists.

# N-API is intentionally the one host-toolchain exception: its native build
# runs on the trusted runner, not inside the manylinux image. Keep that host
# compiler identity separate from the image-owned Python wheel toolchain.
export CUDA_NAPI_HOST_TOOLKIT_ROOT='/usr/local/cuda-12.6'
export CUDA_NAPI_HOST_NVCC_VERSION='Cuda compilation tools, release 12.6, V12.6.68'
export CUDA_NAPI_HOST_GCC_VERSION='13.3.0'
export CUDA_NAPI_HOST_CC='/usr/bin/gcc-13'
export CUDA_NAPI_HOST_CXX='/usr/bin/g++-13'
export CUDA_MANYLINUX='2_28'
# CUDA 7.5 is the lowest device in the restricted runner group (the hosted
# T4). The self-hosted RTX 3090 is forward-compatible with this build target.
export CUDA_COMPUTE_CAP='75'
export CUDA_NAPI_FEATURES='embed-cuda'
export CUDA_PYTHON_FEATURES='pyo3/extension-module,embed-cuda'
# Additive candidate-only Slice 71 tuple. The established v2 route continues
# to use the two variables above; callers opt into this tuple explicitly.
export CUDA_RERANK_NAPI_FEATURES='embed-cuda,rerank-cuda'
export CUDA_RERANK_PYTHON_FEATURES='pyo3/extension-module,embed-cuda,rerank-cuda'
# This designated-runner-only image is intentionally a local image tag, not a
# host-toolchain proxy. It contains the CUDA 12.6 toolkit, Rust, maturin, and
# the manylinux_2_28 Python ABI used by the release build. The preflight fails
# closed when the runner has not provisioned it.
export CUDA_MANYLINUX_IMAGE='fathomdb-cuda-manylinux:12.6-manylinux_2_28'
export CUDA_MANYLINUX_PYTHON='/opt/python/cp311-cp311/bin/python'
export CUDA_MANYLINUX_PLATFORM='linux/amd64'
export CUDA_TOOLKIT_VERSION='12.6.3'
export CUDA_MANYLINUX_PYTHON_ABI='cp311-cp311'
# The two stage bases below are digest-pinned so provisioners do not silently
# rebuild the designated release image from a moved tag. The manylinux image
# supplies the policy toolchain and CPython; CUDA is copied from NVIDIA's
# CUDA 12.6.3 Rocky Linux 8 development image.
export CUDA_MANYLINUX_BASE_IMAGE='quay.io/pypa/manylinux_2_28_x86_64@sha256:aba9efd7dec389abd76506219e461014015b1c1cb95f2a36f27946128910dd07'
export CUDA_TOOLKIT_IMAGE='nvidia/cuda:12.6.3-devel-rockylinux8@sha256:83bc2b9fcf3ab1a4e324f81e962b58957370fa71f7ac61e3a24af399a0ba7595'
export CUDA_RUST_VERSION='1.95.0'
# The containerized wheel build runs as the host runner user. Pin the resolved
# Rustup toolchain so its proxy does not attempt a channel sync under the
# immutable image-owned RUSTUP_HOME.
export CUDA_RUSTUP_TOOLCHAIN='1.95.0-x86_64-unknown-linux-gnu'
export CUDA_MATURIN_VERSION='1.14.1'
export CUDA_MANYLINUX_GCC_TOOLSET='gcc-toolset-13'
export CUDA_MANYLINUX_GCC_VERSION='13.3.1'
export CUDA_MANYLINUX_GCC_ROOT='/opt/rh/gcc-toolset-13/root/usr'
export CUDA_MANYLINUX_CC='/opt/rh/gcc-toolset-13/root/usr/bin/gcc'
export CUDA_MANYLINUX_CXX='/opt/rh/gcc-toolset-13/root/usr/bin/g++'
# `cudarc` dynamic-loading deliberately omits its dynamic-linking search paths,
# while Candle kernels still link the CUDA runtime and libstdc++. Keep both
# image-owned locations explicit without reintroducing a driver-library link.
export CUDA_MANYLINUX_CUDA_LIB64='/usr/local/cuda-12.6/lib64'
export CUDA_MANYLINUX_GCC_LIB='/opt/rh/gcc-toolset-13/root/usr/lib/gcc/x86_64-redhat-linux/13'
export CUDA_MANYLINUX_GCC_RPM='gcc-toolset-13-gcc-13.3.1-2.2.el8_10.x86_64'
export CUDA_MANYLINUX_GXX_RPM='gcc-toolset-13-gcc-c++-13.3.1-2.2.el8_10.x86_64'
export CUDA_MANYLINUX_DOCKERFILE='scripts/release/Dockerfile.cuda-manylinux'
# rustup-init is a pinned bootstrap binary. Rustup subsequently resolves the
# explicitly-versioned Rust toolchain from static.rust-lang.org at image build
# time, so the Docker build is reproducible in inputs but not fully hermetic.
export CUDA_RUSTUP_INIT_URL='https://static.rust-lang.org/rustup/archive/1.29.0/x86_64-unknown-linux-gnu/rustup-init'
export CUDA_RUSTUP_INIT_SHA256='4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10'
# Separate minimal images keep the CPU fallback proof honest: neither image
# receives a GPU device and both execute with `--network none`.
export CUDA_DRIVERLESS_PYTHON_IMAGE='python:3.11-slim'
export CUDA_DRIVERLESS_NODE_IMAGE='node:25-bookworm-slim'
export CUDA_DEFAULT_EMBEDDER_HF_REPO='BAAI/bge-small-en-v1.5'
export CUDA_DEFAULT_EMBEDDER_HF_REVISION='5c38ec7c405ec4b44b94cc5a9bb96e735b38267a'
export CUDA_DEFAULT_EMBEDDER_CONFIG_SHA256='094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750'
export CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256='d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66'
export CUDA_DEFAULT_EMBEDDER_MODEL_SHA256='3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad'
export CUDA_RERANKER_CONFIG_SHA256='2144195e107cd7ea61556478e7add12986ebfbc3085f924fc0b90c2410604879'
export CUDA_RERANKER_TOKENIZER_SHA256='d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66'
export CUDA_RERANKER_MODEL_SHA256='a0e7364ddf91ff7028f1102e1b91ac7a72e3db4061241bd84efe45c72c9af03a'
