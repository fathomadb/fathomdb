#!/usr/bin/env bash
# Build the interim static Tegra package tree: a PEP 503 index for the verified
# `+tegra` wheels and a link page for the verified Jetson Node tarballs, every
# file under tegra/packages/ with its SHA-256.
#
# Each `--wheel` pairs with the `--version` that follows it, and each
# `--node-tarball` with the `--node-version` that follows it.
set -euo pipefail

out=''
wheels=()
wheel_versions=()
tarballs=()
tarball_versions=()

usage() {
  printf 'usage: %s --out DIRECTORY [--wheel WHEEL --version VERSION+tegra]... [--node-tarball TGZ --node-version X.Y.Z]...\n' "$0" >&2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --wheel) wheels+=("${2:-}"); shift 2 ;;
    --version) wheel_versions+=("${2:-}"); shift 2 ;;
    --node-tarball) tarballs+=("${2:-}"); shift 2 ;;
    --node-version) tarball_versions+=("${2:-}"); shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    *)
      usage
      exit 64
      ;;
  esac
done

if [ -z "$out" ] || [ "${#wheels[@]}" -ne "${#wheel_versions[@]}" ] ||
  [ "${#tarballs[@]}" -ne "${#tarball_versions[@]}" ] ||
  [ $(( ${#wheels[@]} + ${#tarballs[@]} )) -eq 0 ]; then
  usage
  exit 64
fi

check_wheel() {
  local wheel="$1" version="$2" wheel_name metadata_file
  [ -f "$wheel" ] || { printf 'wheel is absent: %s\n' "$wheel" >&2; exit 1; }
  wheel_name="$(basename "$wheel")"
  case "$wheel_name" in
    "fathomdb-${version}-"*-linux_aarch64.whl) ;;
    *)
      printf 'wheel filename must be fathomdb-%s-*-linux_aarch64.whl: %s\n' \
        "$version" "$wheel_name" >&2
      exit 1
      ;;
  esac
  metadata_file="$(mktemp)"
  python3 - "$wheel" > "$metadata_file" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    metadata = {
        name for name in archive.namelist() if name.endswith('.dist-info/METADATA')
    }
    if len(metadata) != 1:
        raise SystemExit(f'expected exactly one wheel METADATA file, found {len(metadata)}')
    print(archive.read(metadata.pop()).decode('utf-8'), end='')
PY
  if ! grep -Fx "Version: ${version}" "$metadata_file" >/dev/null; then
    rm -f "$metadata_file"
    printf 'wheel metadata must contain Version: %s\n' "$version" >&2
    exit 1
  fi
  if ! grep -Fx 'Name: fathomdb' "$metadata_file" >/dev/null; then
    rm -f "$metadata_file"
    printf 'wheel metadata must contain Name: fathomdb\n' >&2
    exit 1
  fi
  rm -f "$metadata_file"
}

check_tarball() {
  local tarball="$1" version="$2" tarball_name
  [ -f "$tarball" ] || { printf 'Node tarball is absent: %s\n' "$tarball" >&2; exit 1; }
  tarball_name="$(basename "$tarball")"
  if [ "$tarball_name" != "fathomdb-linux-arm64-gnu-tegra-${version}.tgz" ]; then
    printf 'Node tarball filename must be fathomdb-linux-arm64-gnu-tegra-%s.tgz: %s\n' \
      "$version" "$tarball_name" >&2
    exit 1
  fi
  python3 - "$tarball" "$version" <<'PY'
import json
import sys
import tarfile

path, version = sys.argv[1:]
with tarfile.open(path, 'r:gz') as archive:
    names = set(archive.getnames())
    if 'package/fathomdb.linux-arm64-gnu.node' not in names:
        raise SystemExit('Node tarball lacks package/fathomdb.linux-arm64-gnu.node')
    manifest = json.load(archive.extractfile('package/package.json'))
if manifest.get('name') != 'fathomdb-linux-arm64-gnu-tegra':
    raise SystemExit('Node tarball package name must be fathomdb-linux-arm64-gnu-tegra')
if manifest.get('version') != version:
    raise SystemExit(f"Node tarball package version must be {version}, got {manifest.get('version')}")
PY
}

for index in "${!wheels[@]}"; do
  check_wheel "${wheels[$index]}" "${wheel_versions[$index]}"
done
for index in "${!tarballs[@]}"; do
  check_tarball "${tarballs[$index]}" "${tarball_versions[$index]}"
done

if [ -e "$out" ]; then
  existing_output="$(find "$out" -mindepth 1 -print -quit)"
  if [ -n "$existing_output" ]; then
    printf 'refusing to replace a nonempty Pages output directory: %s\n' "$out" >&2
    exit 1
  fi
fi

mkdir -p "$out/tegra/packages"
install_unique() {
  local file="$1" name
  name="$(basename "$file")"
  if [ -e "$out/tegra/packages/$name" ]; then
    printf 'duplicate Tegra package filename: %s\n' "$name" >&2
    exit 1
  fi
  install -m 0644 "$file" "$out/tegra/packages/$name"
}

if [ "${#wheels[@]}" -gt 0 ]; then
  mkdir -p "$out/tegra/simple/fathomdb"
  cat > "$out/tegra/simple/index.html" <<'HTML'
<!doctype html>
<html><body><a href="fathomdb/">fathomdb</a></body></html>
HTML
  links=''
  for wheel in "${wheels[@]}"; do
    install_unique "$wheel"
    name="$(basename "$wheel")"
    digest="$(sha256sum "$wheel" | awk '{print $1}')"
    links="${links}<a href=\"../../packages/${name}#sha256=${digest}\">${name}</a><br>"$'\n'
  done
  printf '<!doctype html>\n<html><body>\n%s</body></html>\n' "$links" \
    > "$out/tegra/simple/fathomdb/index.html"
fi

if [ "${#tarballs[@]}" -gt 0 ]; then
  mkdir -p "$out/tegra/node"
  links=''
  for tarball in "${tarballs[@]}"; do
    install_unique "$tarball"
    name="$(basename "$tarball")"
    digest="$(sha256sum "$tarball" | awk '{print $1}')"
    links="${links}<a href=\"../packages/${name}#sha256=${digest}\">${name}</a><br>"$'\n'
  done
  printf '<!doctype html>\n<html><body>\n%s</body></html>\n' "$links" \
    > "$out/tegra/node/index.html"
fi

for wheel in "${wheels[@]}"; do
  digest="$(sha256sum "$wheel" | awk '{print $1}')"
  printf 'wheel_sha256=%s\n' "$digest"
done
for tarball in "${tarballs[@]}"; do
  digest="$(sha256sum "$tarball" | awk '{print $1}')"
  printf 'node_tarball_sha256=%s\n' "$digest"
done
