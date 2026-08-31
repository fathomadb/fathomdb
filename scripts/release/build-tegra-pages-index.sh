#!/usr/bin/env bash
# Build the interim static PEP 503 tree for one verified Tegra wheel.
set -euo pipefail

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
cleanup() { rm -f "$metadata_file"; }
trap cleanup EXIT
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
grep -Fx "Version: ${version}" "$metadata_file" >/dev/null || {
  printf 'wheel metadata must contain Version: %s\n' "$version" >&2
  exit 1
}
grep -Fx 'Name: fathomdb' "$metadata_file" >/dev/null || {
  printf 'wheel metadata must contain Name: fathomdb\n' >&2
  exit 1
}

if [ -e "$out" ]; then
  existing_output="$(find "$out" -mindepth 1 -print -quit)"
  if [ -n "$existing_output" ]; then
    printf 'refusing to replace a nonempty Pages output directory: %s\n' "$out" >&2
    exit 1
  fi
fi

wheel_sha256="$(sha256sum "$wheel" | awk '{print $1}')"
mkdir -p "$out/tegra/simple/fathomdb" "$out/tegra/packages"
install -m 0644 "$wheel" "$out/tegra/packages/$wheel_name"
cat > "$out/tegra/simple/index.html" <<'HTML'
<!doctype html>
<html><body><a href="fathomdb/">fathomdb</a></body></html>
HTML
wheel_href="../../packages/${wheel_name}#sha256=${wheel_sha256}"
printf '<!doctype html>\n<html><body><a href="%s">%s</a></body></html>\n' \
  "$wheel_href" "$wheel_name" > "$out/tegra/simple/fathomdb/index.html"
printf 'wheel_sha256=%s\n' "$wheel_sha256"
