#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
if [[ $# -ne 1 || "$1" != /* || -e "$1" || -L "$1" ]]; then
  echo "usage: package.sh ABSOLUTE_NEW_OUTPUT_DIRECTORY" >&2
  exit 2
fi
platform="$(uname -sm)"
if [[ "$platform" != "Linux x86_64" ]]; then
  echo "unsupported_platform: package target is Linux x86_64" >&2
  exit 2
fi
cargo_toml="$root/src/inspector/Cargo.toml"
if [[ ! -f "$cargo_toml" ]]; then
  echo "missing Inspector Cargo.toml: $cargo_toml" >&2
  exit 2
fi
version="$(awk '
  /^\[/ { in_pkg = ($0 == "[package]") }
  in_pkg && /^version[[:space:]]*=/ {
    gsub(/^[^"]*"/, "")
    gsub(/".*$/, "")
    print
    exit
  }
' "$cargo_toml")"
if [[ -z "$version" ]]; then
  echo "failed to read [package].version from $cargo_toml" >&2
  exit 2
fi
output="$1"
parent="$(realpath -e -- "$(dirname -- "$output")")"
leaf="$(basename -- "$output")"
if [[ "$leaf" == . || "$leaf" == .. || "$parent/$leaf" != "$output" ]]; then
  echo "output directory must have a canonical existing parent" >&2
  exit 2
fi
mkdir -m 755 -- "$output"
completed=0
cleanup() {
  if [[ "$completed" != 1 ]]; then
    rm -rf -- "$output"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
build_target="${CARGO_TARGET_DIR:-$root/target}"
if [[ "$build_target" != /* ]]; then
  echo 'CARGO_TARGET_DIR must be absolute' >&2
  exit 2
fi
cd -- "$root"
cargo build --quiet --target-dir "$build_target" --release --target x86_64-unknown-linux-gnu --locked --offline --no-default-features --features cached-cpu -p fathomdb-inspector
name="fathomdb-inspector-${version}-linux-x86_64"
stage="$output/$name"
mkdir -m 755 -- "$stage"
install -m 755 -- "$build_target/x86_64-unknown-linux-gnu/release/fathomdb-inspector" "$stage/fathomdb-inspector"
install -m 644 -- LICENSE "$stage/LICENSE"
install -m 644 -- src/inspector/package/README.md "$stage/README.md"
env -i "$stage/fathomdb-inspector" --build-description > "$stage/build-description.json"
if ! grep -Fq '"cached_default_cpu":true' "$stage/build-description.json"; then
  echo "shipping artifact must compile cached CPU support" >&2
  exit 1
fi
if ! grep -Fq '"fixture_control":false' "$stage/build-description.json"; then
  echo "default artifact must exclude fixture control" >&2
  exit 1
fi
(
  cd -- "$stage"
  sha256sum -- fathomdb-inspector build-description.json LICENSE README.md > SHA256SUMS
)
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime=@0 -czf "$output/$name.tar.gz" -C "$output" "$name"
(
  cd -- "$output"
  sha256sum -- "$name.tar.gz" > SHA256SUMS
)
completed=1
printf '%s\n' "$output/$name.tar.gz"
