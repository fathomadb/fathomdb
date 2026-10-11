#!/usr/bin/env bash
# Prove an extracted Tegra extension has no unresolved CUDA Runtime imports or
# dynamic CUDA/NVIDIA dependencies before the wheel can leave its build stage.
set -euo pipefail

if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
  printf 'check-tegra-wheel-linkage: expected one extracted extension file\n' >&2
  exit 2
fi
extension="$1"

if ! command -v nm >/dev/null 2>&1 || ! command -v readelf >/dev/null 2>&1; then
  printf 'check-tegra-wheel-linkage: nm and readelf are required\n' >&2
  exit 1
fi

if ! undefined_symbols="$(nm -D --undefined-only "$extension")"; then
  printf 'check-tegra-wheel-linkage: nm could not inspect %s\n' "$extension" >&2
  exit 1
fi
if grep -E '^[[:space:]]*U[[:space:]]+(__cuda|cuda)' <<< "$undefined_symbols"; then
  printf 'check-tegra-wheel-linkage: unresolved CUDA Runtime symbols remain\n' >&2
  exit 1
fi

if ! dynamic_entries="$(readelf -d "$extension")"; then
  printf 'check-tegra-wheel-linkage: readelf could not inspect %s\n' "$extension" >&2
  exit 1
fi
if grep -Ei 'Shared library: \[(libcu(da|blas|dnn|fft|rand|solver|sparse|pti|tensor|file|inj)|libnv)' <<< "$dynamic_entries"; then
  printf 'check-tegra-wheel-linkage: dynamic CUDA/NVIDIA dependencies remain\n' >&2
  exit 1
fi
