#!/usr/bin/env bash
# 0.8.23's one checked source for the Linux CUDA artifact build inputs.
#
# It is sourced only by release preflight/build helpers. CPU CI and every
# non-Linux release lane deliberately retain their existing feature lists.

export CUDA_TOOLKIT_ROOT='/usr/local/cuda-12.6'
export CUDA_MANYLINUX='2_28'
# CUDA 7.5 is the lowest device in the restricted runner group (the hosted
# T4). The self-hosted RTX 3090 is forward-compatible with this build target.
export CUDA_COMPUTE_CAP='75'
export CUDA_NAPI_FEATURES='embed-cuda'
export CUDA_PYTHON_FEATURES='pyo3/extension-module,embed-cuda'
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
export CUDA_MATURIN_VERSION='1.14.1'
export CUDA_MANYLINUX_DOCKERFILE='scripts/release/Dockerfile.cuda-manylinux'
# Separate minimal images keep the CPU fallback proof honest: neither image
# receives a GPU device and both execute with `--network none`.
export CUDA_DRIVERLESS_PYTHON_IMAGE='python:3.11-slim'
export CUDA_DRIVERLESS_NODE_IMAGE='node:25-bookworm-slim'
export CUDA_DEFAULT_EMBEDDER_HF_REPO='BAAI/bge-small-en-v1.5'
export CUDA_DEFAULT_EMBEDDER_HF_REVISION='5c38ec7c405ec4b44b94cc5a9bb96e735b38267a'
export CUDA_DEFAULT_EMBEDDER_CONFIG_SHA256='094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750'
export CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256='d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66'
export CUDA_DEFAULT_EMBEDDER_MODEL_SHA256='3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad'
