#!/usr/bin/env bash
# Build the public documentation and the Tegra package tree as one site.
#
# Every Pages deployment replaces the whole site, so the build always starts
# from the retained-artifact list: each listed wheel and Node tarball must be
# present in --artifact-dir with its pinned SHA-256, and all of them are
# republished. A Jetson publication adds its new wheel and tarball beside them;
# a new artifact may not replace a retained one under the same filename.
#
# Prints `retained_artifacts=N`, then `wheel_sha256=` and
# `node_tarball_sha256=` for the new artifacts, if any.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INDEX_BUILDER="$SCRIPT_DIR/build-tegra-pages-index.sh"
RETAINED_HELPER="$SCRIPT_DIR/tegra-pages-retained.py"
manifest=''
artifact_dir=''
wheel=''
version=''
node_tarball=''
node_version=''
out=''

usage() {
  printf 'usage: %s --manifest RETAINED_JSON --artifact-dir DIRECTORY --out DIRECTORY [--wheel WHEEL --version VERSION+tegra] [--node-tarball TGZ --node-version X.Y.Z]\n' "$0" >&2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest) manifest="${2:-}"; shift 2 ;;
    --artifact-dir) artifact_dir="${2:-}"; shift 2 ;;
    --wheel) wheel="${2:-}"; shift 2 ;;
    --version) version="${2:-}"; shift 2 ;;
    --node-tarball) node_tarball="${2:-}"; shift 2 ;;
    --node-version) node_version="${2:-}"; shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    *)
      usage
      exit 64
      ;;
  esac
done

if [ -z "$manifest" ] || [ -z "$artifact_dir" ] || [ -z "$out" ] ||
  { [ -n "$wheel" ] && [ -z "$version" ]; } || { [ -z "$wheel" ] && [ -n "$version" ]; } ||
  { [ -n "$node_tarball" ] && [ -z "$node_version" ]; } ||
  { [ -z "$node_tarball" ] && [ -n "$node_version" ]; }; then
  usage
  exit 64
fi
[ -d "$artifact_dir" ] || { printf 'retained artifact directory is absent: %s\n' "$artifact_dir" >&2; exit 1; }
[ -z "$wheel" ] || [ -f "$wheel" ] || { printf 'wheel is absent: %s\n' "$wheel" >&2; exit 1; }
[ -z "$node_tarball" ] || [ -f "$node_tarball" ] ||
  { printf 'Node tarball is absent: %s\n' "$node_tarball" >&2; exit 1; }
command -v mkdocs >/dev/null || { printf 'mkdocs is not installed\n' >&2; exit 1; }

if [ -e "$out" ]; then
  existing_output="$(find "$out" -mindepth 1 -print -quit)"
  if [ -n "$existing_output" ]; then
    printf 'refusing to replace a nonempty Pages output directory: %s\n' "$out" >&2
    exit 1
  fi
fi

listing="$(python3 "$RETAINED_HELPER" list --manifest "$manifest")"
index_args=()
retained_count=0
declare -A retained_sha256=()
while IFS=$'\t' read -r kind artifact_version filename sha256 _url; do
  [ -n "$filename" ] || continue
  file="$artifact_dir/$filename"
  [ -f "$file" ] || { printf 'retained artifact is absent from %s: %s\n' "$artifact_dir" "$filename" >&2; exit 1; }
  actual="$(sha256sum "$file" | awk '{print $1}')"
  if [ "$actual" != "$sha256" ]; then
    printf 'retained artifact SHA-256 mismatch for %s: expected %s, got %s\n' "$filename" "$sha256" "$actual" >&2
    exit 1
  fi
  retained_sha256["$filename"]="$sha256"
  case "$kind" in
    wheel) index_args+=(--wheel "$file" --version "$artifact_version") ;;
    node-tarball) index_args+=(--node-tarball "$file" --node-version "$artifact_version") ;;
    *) printf 'unknown retained artifact kind: %s\n' "$kind" >&2; exit 1 ;;
  esac
  retained_count=$((retained_count + 1))
done <<<"$listing"

# A new artifact already on the list is the same publication rebuilt and must
# be byte-identical; otherwise it would silently replace a published URL.
add_new() {
  local file="$1" flag="$2" version_flag="$3" artifact_version="$4" name digest
  name="$(basename "$file")"
  digest="$(sha256sum "$file" | awk '{print $1}')"
  if [ -n "${retained_sha256[$name]:-}" ]; then
    if [ "${retained_sha256[$name]}" != "$digest" ]; then
      printf 'new artifact %s differs from the retained, already published %s\n' "$name" "$name" >&2
      exit 1
    fi
    return 0
  fi
  index_args+=("$flag" "$file" "$version_flag" "$artifact_version")
}
[ -z "$wheel" ] || add_new "$wheel" --wheel --version "$version"
[ -z "$node_tarball" ] || add_new "$node_tarball" --node-tarball --node-version "$node_version"
[ "${#index_args[@]}" -gt 0 ] || { printf 'no Tegra artifact to publish\n' >&2; exit 1; }

tegra_root="$(mktemp -d)"
cleanup() {
  case "$tegra_root" in
    "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$tegra_root" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$tegra_root" >&2 ;;
  esac
}
trap cleanup EXIT

bash "$INDEX_BUILDER" --out "$tegra_root" "${index_args[@]}" >&2
cd "$REPO_ROOT"
python3 "$REPO_ROOT/scripts/generate-public-docs.py" --check >&2
mkdocs build --strict --site-dir "$out" >&2
cp -R "$tegra_root/tegra" "$out/tegra"

printf 'retained_artifacts=%s\n' "$retained_count"
if [ -n "$wheel" ]; then
  digest="$(sha256sum "$wheel" | awk '{print $1}')"
  printf 'wheel_sha256=%s\n' "$digest"
fi
if [ -n "$node_tarball" ]; then
  digest="$(sha256sum "$node_tarball" | awk '{print $1}')"
  printf 'node_tarball_sha256=%s\n' "$digest"
fi
