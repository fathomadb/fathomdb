#!/usr/bin/env bash
# Resolve one immutable host CUDA UUID to its current NVML index.

resolve_cuda_gpu_index() {
  local requested_uuid="$1" line index uuid matched_index='' matches=0
  if ! [[ "$requested_uuid" =~ ^GPU-[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$ ]]; then
    printf 'cuda-gpu-selection: FATHOMDB_CUDA_GPU_UUID must be a canonical GPU UUID\n' >&2
    return 1
  fi
  while IFS=, read -r index uuid; do
    index="${index// /}"
    uuid="${uuid// /}"
    if [ "$uuid" = "$requested_uuid" ]; then
      matched_index="$index"
      matches=$((matches + 1))
    fi
  done < <(nvidia-smi --query-gpu=index,uuid --format=csv,noheader)
  if [ "$matches" -ne 1 ] || ! [[ "$matched_index" =~ ^[0-9]+$ ]]; then
    printf 'cuda-gpu-selection: requested GPU UUID must resolve exactly once on this host\n' >&2
    return 1
  fi
  CUDA_GPU_INDEX="$matched_index"
}
