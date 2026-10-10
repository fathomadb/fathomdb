#!/usr/bin/env bash
# Download every artifact in the retained Tegra Pages list from its published
# URL and fail closed unless each matches its pinned SHA-256. Both Pages
# deployers run this so neither can drop a published wheel or Node tarball.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELPER="$SCRIPT_DIR/tegra-pages-retained.py"
manifest=''
dest=''

usage() {
  printf 'usage: %s --manifest RETAINED_JSON --dest DIRECTORY\n' "$0" >&2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest) manifest="${2:-}"; shift 2 ;;
    --dest) dest="${2:-}"; shift 2 ;;
    *) usage; exit 64 ;;
  esac
done
[ -n "$manifest" ] && [ -n "$dest" ] || { usage; exit 64; }

listing="$(python3 "$HELPER" list --manifest "$manifest")"
mkdir -p "$dest"
count=0
while IFS=$'\t' read -r kind version filename sha256 url; do
  [ -n "$filename" ] || continue
  target="$dest/$filename"
  curl --fail --location --proto '=https' --tlsv1.2 \
    --retry 4 --retry-all-errors "$url" --output "$target"
  actual="$(sha256sum "$target" | awk '{print $1}')"
  if [ "$actual" != "$sha256" ]; then
    printf 'retained %s %s SHA-256 mismatch: expected %s, got %s\n' \
      "$kind" "$version" "$sha256" "$actual" >&2
    exit 1
  fi
  count=$((count + 1))
done <<<"$listing"
printf 'retained_artifacts=%s\n' "$count"
