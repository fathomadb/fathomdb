#!/usr/bin/env bash
# Build the public documentation and retained Tegra package index as one site.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INDEX_BUILDER="$SCRIPT_DIR/build-tegra-pages-index.sh"
wheel=''
out=''
version=''

usage() {
  printf 'usage: %s --wheel WHEEL --out DIRECTORY --version VERSION+tegra\n' "$0" >&2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --wheel)
      wheel="${2:-}"
      shift 2
      ;;
    --out)
      out="${2:-}"
      shift 2
      ;;
    --version)
      version="${2:-}"
      shift 2
      ;;
    *)
      usage
      exit 64
      ;;
  esac
done

[ -n "$wheel" ] && [ -n "$out" ] && [ -n "$version" ] || {
  usage
  exit 64
}
[ -f "$wheel" ] || { printf 'wheel is absent: %s\n' "$wheel" >&2; exit 1; }
command -v mkdocs >/dev/null || { printf 'mkdocs is not installed\n' >&2; exit 1; }

if [ -e "$out" ]; then
  existing_output="$(find "$out" -mindepth 1 -print -quit)"
  if [ -n "$existing_output" ]; then
    printf 'refusing to replace a nonempty Pages output directory: %s\n' "$out" >&2
    exit 1
  fi
fi

tegra_root="$(mktemp -d)"
cleanup() {
  case "$tegra_root" in
    "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$tegra_root" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$tegra_root" >&2 ;;
  esac
}
trap cleanup EXIT

cd "$REPO_ROOT"
mkdocs build --strict --site-dir "$out" >&2
index_result="$(bash "$INDEX_BUILDER" \
  --wheel "$wheel" --out "$tegra_root" --version "$version")"
cp -R "$tegra_root/tegra" "$out/tegra"
printf '%s\n' "$index_result"
