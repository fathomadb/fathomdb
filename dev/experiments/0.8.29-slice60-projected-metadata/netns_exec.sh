#!/usr/bin/env bash
# Reviewed manual qualification runtime boundary; never runs on the host netns.
set -euo pipefail
mode="$1"; host_netns="$2"; binary="$3"; run_root="$4"
task="${S60_TASK:?S60_TASK is required}"
source_root="${S60_SOURCE:?S60_SOURCE is required}"
export CUDA_VISIBLE_DEVICES="${S60_GPU_UUID:?S60_GPU_UUID is required}"
export FATHOMDB_EMBED_DEVICE='cuda:0'
export XDG_CACHE_HOME="$task/gpu-metadata/model-cache"
export HF_HOME="$task/gpu-metadata/hf-home"
export HF_HUB_CACHE="$HF_HOME/hub"
export CUDA_CACHE_PATH="$task/gpu-metadata/cuda-cache"
export CUDA_PATH='/usr/local/cuda-12.6'
export LD_LIBRARY_PATH='/usr/local/cuda-12.6/targets/x86_64-linux/lib'
unset HF_TOKEN HUGGING_FACE_HUB_TOKEN HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
unset FATHOMDB_GPU_ALLOCATION_WITNESS
exec "$source_root/.venv/bin/python" -I "$(dirname -- "$0")/netns_final_exec.py" "$mode" "$host_netns" "$binary" "$run_root"
