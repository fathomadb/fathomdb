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
export CUDA_DRIVERLESS_IMAGE='python:3.11-slim'
