#!/usr/bin/env bash
# Source this file and call select_cuda_rehearsal_packages DIR VERSION.
# The caller receives wheel, npm_main, and napi_platform only for one exact
# candidate package inventory. Self-hosted runner workspaces can retain files.

select_cuda_rehearsal_packages() {
  local dir="${1:-}" version="${2:-}" path entry count=0
  [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
    && [ -d "$dir" ] && [ ! -L "$dir" ] || {
    printf 'cuda-rehearsal-packages: invalid directory or version: %s %s\n' "$dir" "$version" >&2
    return 1
  }
  wheel="$dir/fathomdb-$version-cp310-abi3-manylinux_2_28_x86_64.whl"
  npm_main="$dir/fathomdb-$version.tgz"
  napi_platform="$dir/fathomdb-linux-x64-gnu-$version.tgz"
  for path in "$wheel" "$npm_main" "$napi_platform"; do
    [ -f "$path" ] && [ ! -L "$path" ] || {
      printf 'cuda-rehearsal-packages: required candidate package absent: %s\n' "$path" >&2
      return 1
    }
  done
  for entry in "$dir"/* "$dir"/.[!.]* "$dir"/..?*; do
    if [ -e "$entry" ] || [ -L "$entry" ]; then
      count=$((count + 1))
    fi
  done
  [ "$count" -eq 3 ] || {
    printf 'cuda-rehearsal-packages: expected exactly 3 candidate packages, found %s in %s\n' "$count" "$dir" >&2
    return 1
  }
}
