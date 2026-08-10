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
# Separate minimal images keep the CPU fallback proof honest: neither image
# receives a GPU device and both execute with `--network none`.
export CUDA_DRIVERLESS_PYTHON_IMAGE='python:3.11-slim'
export CUDA_DRIVERLESS_NODE_IMAGE='node:25-bookworm-slim'
export CUDA_DEFAULT_EMBEDDER_HF_REPO='BAAI/bge-small-en-v1.5'
export CUDA_DEFAULT_EMBEDDER_HF_REVISION='5c38ec7c405ec4b44b94cc5a9bb96e735b38267a'
