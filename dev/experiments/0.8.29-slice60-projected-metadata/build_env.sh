#!/usr/bin/env bash
# Source after the checkout-owned base environment; no host configuration.
export CARGO_TARGET_DIR=${S60_TASK}/gpu-metadata/target
export XDG_CACHE_HOME=${S60_TASK}/gpu-metadata/build-cache
export CUDA_VISIBLE_DEVICES="${S60_GPU_UUID:?S60_GPU_UUID is required}"
export CUDA_PATH=/usr/local/cuda-12.6
export CUDACXX=/usr/local/cuda-12.6/bin/nvcc
export CC=/usr/bin/gcc-13
export CXX=/usr/bin/g++-13
export CUDAHOSTCXX=/usr/bin/g++-13
export NVCC_CCBIN=/usr/bin/g++-13
export LIBRARY_PATH=/usr/local/cuda-12.6/targets/x86_64-linux/lib
export PATH=/usr/local/cuda-12.6/bin:$PATH
export CUDA_COMPUTE_CAP=75
export CUDAFORGE_THREADS=2
export RAYON_NUM_THREADS=2
