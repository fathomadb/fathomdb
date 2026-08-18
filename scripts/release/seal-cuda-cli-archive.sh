#!/usr/bin/env bash
# Seal one deterministic Linux x86_64 CLI archive for rehearsal only.
set -euo pipefail

usage() { printf 'usage: %s --binary FILE --version VERSION --output FILE\n' "$0" >&2; }
binary='' version='' output=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --binary) binary="$2" ;;
    --version) version="$2" ;;
    --output) output="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done
[ -f "$binary" ] && [ ! -L "$binary" ] || { printf 'seal-cuda-cli: binary must be a regular non-symlink file\n' >&2; exit 1; }
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { printf 'seal-cuda-cli: invalid version\n' >&2; exit 1; }
[ -n "$output" ] && [ ! -e "$output" ] || { printf 'seal-cuda-cli: output must be a new path\n' >&2; exit 1; }
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
root="fathomdb-$version-x86_64-unknown-linux-gnu"
install -d -m 0755 "$stage/$root"
install -m 0755 "$binary" "$stage/$root/fathomdb"
umask 022
tar --format=posix --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  --pax-option=delete=atime,delete=ctime --mode='u=rwx,go=rx' \
  -C "$stage" -cf - "$root" | gzip -n > "$output"
