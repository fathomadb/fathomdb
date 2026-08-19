#!/usr/bin/env bash
# 0.8.23's one checked source for the Linux CUDA artifact build inputs.
#
# It is sourced only by release preflight/build helpers. CPU CI and every
# non-Linux release lane deliberately retain their existing feature lists.

# N-API is intentionally the one host-toolchain exception: its native build
# runs on the trusted runner, not inside the manylinux image. Keep that host
# compiler identity separate from the image-owned Python wheel toolchain.
#
# 0.8.23 Slice 80.6 (D-80.6-4): the Jetson/Tegra host-native wheel build is the
# second host-toolchain consumer. The toolkit root and the nvcc version pin are
# MEASURED IDENTICAL on the Jetson Orin AGX (design § 2.7), so both targets
# share these two literals by reference below rather than declaring a second
# copy that could drift.
export CUDA_NAPI_HOST_TOOLKIT_ROOT='/usr/local/cuda-12.6'
export CUDA_NAPI_HOST_NVCC_VERSION='Cuda compilation tools, release 12.6, V12.6.68'
# Host compiler as a per-target axis, exactly as 80.4 did for
# CUDA_COMPUTE_CAP: sibling per-target constants plus a selector, with the
# x86_64 literals byte-identical to what this contract has always pinned
# (AC80-8). This is the one genuine per-target toolchain split — the Jetson
# carries stock Ubuntu 22.04 gcc 11.4.0 and has no gcc-13, and CUDA 12.6
# supports gcc 11 (measured: `cargo build -p fathomdb-embedder --features
# embed-cuda` succeeds on this Orin with it, design § 2.7).
export CUDA_HOST_GCC_VERSION_X86_64='13.3.0'
export CUDA_HOST_CC_X86_64='/usr/bin/gcc-13'
export CUDA_HOST_CXX_X86_64='/usr/bin/g++-13'
export CUDA_HOST_GCC_VERSION_TEGRA_ORIN='11.4.0'
export CUDA_HOST_CC_TEGRA_ORIN='/usr/bin/gcc'
export CUDA_HOST_CXX_TEGRA_ORIN='/usr/bin/g++'
# The N-API host-toolchain names keep their meaning and their values; they now
# select the x86_64 axis, because that is the only target build-napi-cuda.sh
# and cuda-preflight.sh build for (D-80.6-2 scopes Tegra to the Python wheel).
export CUDA_NAPI_HOST_GCC_VERSION="$CUDA_HOST_GCC_VERSION_X86_64"
export CUDA_NAPI_HOST_CC="$CUDA_HOST_CC_X86_64"
export CUDA_NAPI_HOST_CXX="$CUDA_HOST_CXX_X86_64"
# Tegra host-native build inputs (Slice 80.6, D-80.6-4), all measured on this
# Jetson Orin AGX rather than assumed:
#   - the toolkit root and nvcc pin are SHARED with x86_64, by reference;
#   - the CUDA runtime libraries live under targets/aarch64-linux/lib, with
#     lib64 a symlink to it. Without LIBRARY_PATH naming this, linking fails
#     `cannot find -lcudart` (measured, § 2.7);
#   - the NVIDIA driver library lives in an L4T-specific `nvidia/`
#     subdirectory, not at the bare aarch64 multiarch path, which carries only
#     a `.so` and no `.so.1`.
export CUDA_TEGRA_HOST_TOOLKIT_ROOT="$CUDA_NAPI_HOST_TOOLKIT_ROOT"
export CUDA_TEGRA_HOST_NVCC_VERSION="$CUDA_NAPI_HOST_NVCC_VERSION"
export CUDA_TEGRA_HOST_CUDART_LIB='/usr/local/cuda-12.6/targets/aarch64-linux/lib'
export CUDA_TEGRA_HOST_DRIVER_LIB='/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1'
export CUDA_TEGRA_HOST_ARCH='aarch64'
# The Tegra wheel is host-bound against JetPack's own glibc (2.35, see
# scripts/release/glibc-floor-contract.sh), so it deliberately carries the
# native `linux` platform tag rather than a manylinux one. § 3 / D-80.6-1 keep
# it out of any registry, which is what makes an unauditable tag acceptable
# here; auditwheel is skipped for the same reason rather than silently failed.
export CUDA_TEGRA_WHEEL_COMPATIBILITY='linux'
export CUDA_MANYLINUX='2_28'
# 0.8.23 Slice 80.4 (R80-5): compute capability as a per-target axis rather
# than a single literal, so a Tegra target can declare its own pin without
# touching x86_64's. CUDA 7.5 is the lowest device in the restricted x86_64
# runner group (the hosted T4); the self-hosted RTX 3090 is forward-compatible
# with this build target. CUDA_COMPUTE_CAP_TEGRA_ORIN='87' matches the Jetson
# Orin AGX's SM_87 — Tegra is NOT forward-compatible the way discrete GPUs
# are, so unlike x86_64 this is a single pin, not a list (§ 7 80.4). Neither
# value is consumed by a real build yet: x86_64's compiled kernels don't
# depend on it (candle-flash-attn-v3, which reads it, isn't in this crate
# graph), and Tegra's own host-native build script doesn't exist until
# Slice 80.6.
export CUDA_COMPUTE_CAP_X86_64='75'
export CUDA_COMPUTE_CAP_TEGRA_ORIN='87'
# CUDA_COMPUTE_CAP is the literal env var name candle-flash-attn-v3's
# build.rs (in the pinned Candle fork) reads — not ours to rename. It selects
# the x86_64 value because that is the only target build-napi-cuda.sh and
# cuda-preflight.sh build for today; Tegra's future host-native build script
# will source CUDA_COMPUTE_CAP_TEGRA_ORIN directly instead.
export CUDA_COMPUTE_CAP="$CUDA_COMPUTE_CAP_X86_64"
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
