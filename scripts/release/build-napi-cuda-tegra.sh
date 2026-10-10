#!/usr/bin/env bash
# Build the Jetson/Tegra CUDA N-API addon host-natively from the release
# contract and pack it as the separately named
# `fathomdb-linux-arm64-gnu-tegra` platform package beside the main `fathomdb`
# package (ADR-0.8.27-jetson-tegra-node-addon-distribution).
#
# Host-native for the same reasons as build-python-cuda-tegra.sh: the Tegra
# CUDA runtime is host-bound and no manylinux image carries a Tegra toolkit.
# The `.node` therefore carries the `tegra` glibc floor, not the generic
# linux-arm64-gnu package's manylinux one. This wrapper never touches the
# generic package, its features or its hosted build.
#
# This wrapper BUILDS, PROVES and PACKS. It publishes nothing: the tarball is
# distributed only by the hosted Tegra Pages job, behind a separate release
# publication ruling, and never through the npm registry.
#
# Usage: build-napi-cuda-tegra.sh [--base-version <version>] [--out <dir>] [--assert-only]
#
#   --base-version  Version stamped into both staged package manifests.
#                   Defaults to src/ts/package.json's version. Must be a plain
#                   semver release or prerelease; npm drops build metadata, so
#                   `+tegra` cannot distinguish this artifact.
#   --out           Directory for the two tarballs, the built `.node` and
#                   SHA256SUMS. Defaults to target/tegra-napi-dist.
#   --assert-only   Run every toolchain, host and package-metadata assertion,
#                   print the resolved build environment, and exit 0 without
#                   compiling or packing.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"
# shellcheck source=glibc-floor-contract.sh
. "$SCRIPT_DIR/glibc-floor-contract.sh"

TS_DIR="$REPO_ROOT/src/ts"
TEGRA_PACKAGE_SOURCE="$TS_DIR/npm-tegra/linux-arm64-gnu-tegra"
OUT_DIR="$REPO_ROOT/target/tegra-napi-dist"
ASSERT_ONLY=0
BASE_VERSION=''

usage_error() {
  printf 'build-napi-cuda-tegra: %s\n' "$1" >&2
  printf 'usage: %s [--base-version <version>] [--out <dir>] [--assert-only]\n' "$(basename "$0")" >&2
  exit 2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      [ "$#" -ge 2 ] || usage_error '--out needs a directory'
      OUT_DIR="$2"
      shift 2
      ;;
    --base-version)
      [ "$#" -ge 2 ] || usage_error '--base-version needs a version'
      BASE_VERSION="$2"
      shift 2
      ;;
    --assert-only)
      ASSERT_ONLY=1
      shift
      ;;
    *)
      usage_error "unknown argument: $1"
      ;;
  esac
done

SEMVER_RELEASE='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
if [ -n "$BASE_VERSION" ] && ! [[ "$BASE_VERSION" =~ $SEMVER_RELEASE ]]; then
  usage_error "--base-version must be a plain semver release without build metadata: $BASE_VERSION"
fi

fail() {
  printf 'build-napi-cuda-tegra: %s\n' "$1" >&2
  exit 1
}

# --- Host and toolchain identity, asserted before anything is built ---------

HOST_ARCH="$(uname -m)"
if [ "$HOST_ARCH" != "$CUDA_TEGRA_HOST_ARCH" ]; then
  fail "the Tegra N-API addon is built host-natively on $CUDA_TEGRA_HOST_ARCH; this host is $HOST_ARCH"
fi

if [ ! -x "$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc" ]; then
  fail "expected the JetPack CUDA compiler at $CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc"
fi
if ! "$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc" --version | grep -F "$CUDA_TEGRA_HOST_NVCC_VERSION"; then
  fail 'host CUDA compiler does not match the release contract'
fi

for compiler in "$CUDA_HOST_CC_TEGRA_ORIN" "$CUDA_HOST_CXX_TEGRA_ORIN"; do
  if [ ! -x "$compiler" ]; then
    fail "expected the contracted Tegra host compiler at $compiler"
  fi
done
if ! "$CUDA_HOST_CC_TEGRA_ORIN" --version | grep -F "$CUDA_HOST_GCC_VERSION_TEGRA_ORIN"; then
  fail 'host C compiler does not match the release contract'
fi
if ! "$CUDA_HOST_CXX_TEGRA_ORIN" --version | grep -F "$CUDA_HOST_GCC_VERSION_TEGRA_ORIN"; then
  fail 'host C++ compiler does not match the release contract'
fi

# Without LIBRARY_PATH naming this directory the Candle kernels fail to link
# `-lcudart` on the Orin; the static runtime still leaves no dynamic CUDART
# dependency in the artifact (checked after the build).
if [ ! -f "$CUDA_TEGRA_HOST_CUDART_LIB/libcudart.so" ]; then
  fail "expected the CUDA runtime import library at $CUDA_TEGRA_HOST_CUDART_LIB/libcudart.so"
fi
# The driver is loaded lazily at run time, never linked. Requiring it here
# makes a driverless build host fail now instead of at the first device use.
if [ ! -e "$CUDA_TEGRA_HOST_DRIVER_LIB" ]; then
  fail "expected the L4T NVIDIA driver library at $CUDA_TEGRA_HOST_DRIVER_LIB"
fi

# rustc resolves through rust-toolchain.toml, so ask from the repository root.
if ! rustc_version="$(cd "$REPO_ROOT" && rustc --version)"; then
  fail 'rustc is not available'
fi
case "$rustc_version" in
  "rustc $CUDA_TEGRA_NAPI_RUST_VERSION "*) printf '%s\n' "$rustc_version" ;;
  *) fail "rustc does not match the release contract ($CUDA_TEGRA_NAPI_RUST_VERSION): $rustc_version" ;;
esac

if ! node_version="$(node --version)"; then
  fail 'node is not available'
fi
if [ "$node_version" != "v$CUDA_TEGRA_NAPI_NODE_VERSION" ]; then
  fail "node does not match the release contract (v$CUDA_TEGRA_NAPI_NODE_VERSION): $node_version"
fi
printf 'node %s\n' "$node_version"
if ! npm_version="$(npm --version)"; then
  fail 'npm is not available'
fi
if [ "$npm_version" != "$CUDA_TEGRA_NAPI_NPM_VERSION" ]; then
  fail "npm does not match the release contract ($CUDA_TEGRA_NAPI_NPM_VERSION): $npm_version"
fi
printf 'npm %s\n' "$npm_version"
if ! grep -F "\"packageManager\": \"npm@$CUDA_TEGRA_NAPI_NPM_VERSION\"" "$TS_DIR/package.json" >/dev/null; then
  fail "src/ts/package.json packageManager is not npm@$CUDA_TEGRA_NAPI_NPM_VERSION"
fi
if [ ! -f "$TS_DIR/node_modules/@napi-rs/cli/package.json" ] || [ ! -f "$TS_DIR/node_modules/typescript/package.json" ]; then
  fail 'src/ts dependencies are not installed; run `npm ci` in src/ts first'
fi

# The committed metadata must name the contracted package and the triple's
# binary, and must leave the version to this wrapper.
if ! node -e '
  const [file, name, binary] = process.argv.slice(1);
  const pkg = JSON.parse(require("fs").readFileSync(file, "utf8"));
  const problems = [];
  if (pkg.name !== name) problems.push(`name is ${pkg.name}`);
  if (pkg.main !== binary) problems.push(`main is ${pkg.main}`);
  if (JSON.stringify(pkg.files) !== JSON.stringify([binary])) problems.push("files must list only the binary");
  if ("version" in pkg) problems.push("version must be stamped, not committed");
  if (problems.length) { console.error(problems.join("; ")); process.exit(1); }
' "$TEGRA_PACKAGE_SOURCE/package.json" "$CUDA_TEGRA_NAPI_PACKAGE" "fathomdb.$CUDA_TEGRA_NAPI_TRIPLE.node"; then
  fail "Tegra package metadata at $TEGRA_PACKAGE_SOURCE does not match the release contract"
fi

if ! MAIN_VERSION="$(node -p 'require(process.argv[1]).version' "$TS_DIR/package.json")"; then
  fail 'could not read the main package version'
fi
if [ -z "$BASE_VERSION" ]; then
  BASE_VERSION="$MAIN_VERSION"
fi
if ! [[ "$BASE_VERSION" =~ $SEMVER_RELEASE ]]; then
  fail "the main package version is not a plain semver release: $BASE_VERSION"
fi

# --- Build environment, every value from the contract -----------------------
export CUDA_PATH="$CUDA_TEGRA_HOST_TOOLKIT_ROOT"
export CUDACXX="$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc"
export CC="$CUDA_HOST_CC_TEGRA_ORIN"
export CXX="$CUDA_HOST_CXX_TEGRA_ORIN"
export CUDAHOSTCXX="$CUDA_HOST_CXX_TEGRA_ORIN"
export NVCC_CCBIN="$CUDA_HOST_CXX_TEGRA_ORIN"
# Orin is SM_87 and Tegra is not forward-compatible like discrete GPUs, so
# this is the single Tegra pin, never the x86_64 value.
export CUDA_COMPUTE_CAP="$CUDA_COMPUTE_CAP_TEGRA_ORIN"
export LIBRARY_PATH="$CUDA_TEGRA_HOST_CUDART_LIB${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LD_LIBRARY_PATH="$CUDA_TEGRA_HOST_CUDART_LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin:$PATH"

printf 'build-napi-cuda-tegra: CUDA_PATH=%s\n' "$CUDA_PATH"
printf 'build-napi-cuda-tegra: CC=%s CXX=%s\n' "$CC" "$CXX"
printf 'build-napi-cuda-tegra: CUDA_COMPUTE_CAP=%s\n' "$CUDA_COMPUTE_CAP"
printf 'build-napi-cuda-tegra: LIBRARY_PATH=%s\n' "$LIBRARY_PATH"
printf 'build-napi-cuda-tegra: LD_LIBRARY_PATH=%s\n' "$LD_LIBRARY_PATH"
printf 'build-napi-cuda-tegra: features=%s\n' "$CUDA_TEGRA_NAPI_FEATURES"
printf 'build-napi-cuda-tegra: package=%s triple=%s\n' "$CUDA_TEGRA_NAPI_PACKAGE" "$CUDA_TEGRA_NAPI_TRIPLE"
printf 'build-napi-cuda-tegra: base-version=%s (src/ts/package.json: %s)\n' "$BASE_VERSION" "$MAIN_VERSION"

if [ "$ASSERT_ONLY" -eq 1 ]; then
  printf 'build-napi-cuda-tegra: assertions passed; --assert-only, not building\n'
  exit 0
fi

mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
STAGING_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-tegra-napi.XXXXXX")"
trap 'rm -rf "$STAGING_ROOT"' EXIT
NATIVE_DIR="$STAGING_ROOT/native"
mkdir -p "$NATIVE_DIR"

# napi writes into an explicit staging directory so the CUDA `.node` never
# lands in src/ts, where the loader's local-binary probe would pick it up.
# @napi-rs/cli 2.x joins process.cwd() onto destDir for the type definitions,
# so the destination must be relative to src/ts.
NATIVE_DEST="$(realpath --relative-to="$TS_DIR" "$NATIVE_DIR")"
cd "$TS_DIR"
npm exec --offline -- napi build --platform --release \
  --cargo-cwd ../rust/crates/fathomdb-napi \
  --features "$CUDA_TEGRA_NAPI_FEATURES" \
  --js false \
  "$NATIVE_DEST"
NATIVE_BINARY="$NATIVE_DIR/fathomdb.$CUDA_TEGRA_NAPI_TRIPLE.node"
if [ ! -f "$NATIVE_BINARY" ]; then
  fail "the Tegra N-API build did not produce $NATIVE_BINARY"
fi
printf 'build-napi-cuda-tegra: built %s\n' "$NATIVE_BINARY"

# The floor is declared per artifact family and asserted against the binary
# this build produced, never exempted.
TEGRA_FLOOR="$(glibc_floor_for_family tegra)"
bash "$REPO_ROOT/scripts/check-glibc-floor.sh" --floor "$TEGRA_FLOOR" "$NATIVE_BINARY"
printf 'build-napi-cuda-tegra: %s is within the declared tegra glibc floor %s\n' \
  "$(basename "$NATIVE_BINARY")" "$TEGRA_FLOOR"
# ADR-0.8.25's linkage rules: no unresolved CUDA Runtime symbol and no CUDA or
# NVIDIA NEEDED entry. The checker fails closed if it cannot inspect the file.
bash "$SCRIPT_DIR/check-tegra-wheel-linkage.sh" "$NATIVE_BINARY"
printf 'build-napi-cuda-tegra: %s has no dynamic CUDA/NVIDIA dependency\n' "$(basename "$NATIVE_BINARY")"

# --- Stage and pack both packages -------------------------------------------
stamp_version() {
  node -e '
    const fs = require("fs");
    const [file, version] = process.argv.slice(1);
    const pkg = JSON.parse(fs.readFileSync(file, "utf8"));
    delete pkg["//version"];
    pkg.version = version;
    fs.writeFileSync(file, JSON.stringify(pkg, null, 2) + "\n");
  ' "$1" "$2"
}

MAIN_STAGE="$STAGING_ROOT/main"
TEGRA_STAGE="$STAGING_ROOT/tegra"
mkdir -p "$MAIN_STAGE" "$TEGRA_STAGE"
npm exec --offline -- tsc -p tsconfig.build.json --outDir "$MAIN_STAGE/dist"
cp "$TS_DIR/package.json" "$TS_DIR/LICENSE" "$TS_DIR/README.md" "$MAIN_STAGE/"
stamp_version "$MAIN_STAGE/package.json" "$BASE_VERSION"
cp "$TEGRA_PACKAGE_SOURCE/package.json" "$TEGRA_PACKAGE_SOURCE/README.md" "$TEGRA_STAGE/"
# npm skips a symlinked LICENSE, so ship a real copy of the repository licence.
cp "$REPO_ROOT/LICENSE" "$TEGRA_STAGE/LICENSE"
cp "$NATIVE_BINARY" "$TEGRA_STAGE/"
stamp_version "$TEGRA_STAGE/package.json" "$BASE_VERSION"

MAIN_TARBALL="$OUT_DIR/fathomdb-$BASE_VERSION.tgz"
TEGRA_TARBALL="$OUT_DIR/$CUDA_TEGRA_NAPI_PACKAGE-$BASE_VERSION.tgz"
rm -f "$MAIN_TARBALL" "$TEGRA_TARBALL"
(cd "$MAIN_STAGE" && npm pack --ignore-scripts --pack-destination "$OUT_DIR" >/dev/null)
(cd "$TEGRA_STAGE" && npm pack --ignore-scripts --pack-destination "$OUT_DIR" >/dev/null)
for tarball in "$MAIN_TARBALL" "$TEGRA_TARBALL"; do
  [ -f "$tarball" ] || fail "npm pack did not produce $tarball"
done

# The packed bytes, not only the staged file, must be the binary just proven.
if ! tegra_listing="$(tar -tzf "$TEGRA_TARBALL")"; then
  fail "could not list $TEGRA_TARBALL"
fi
for entry in package/package.json package/LICENSE "package/fathomdb.$CUDA_TEGRA_NAPI_TRIPLE.node"; do
  grep -Fx "$entry" <<<"$tegra_listing" >/dev/null || fail "$TEGRA_TARBALL lacks $entry"
done
PACKED_NATIVE="$STAGING_ROOT/packed.node"
tar -xzOf "$TEGRA_TARBALL" "package/fathomdb.$CUDA_TEGRA_NAPI_TRIPLE.node" >"$PACKED_NATIVE"
cmp -s "$PACKED_NATIVE" "$NATIVE_BINARY" || fail 'the packed Tegra binary differs from the proven build output'
if ! main_listing="$(tar -tzf "$MAIN_TARBALL")"; then
  fail "could not list $MAIN_TARBALL"
fi
grep -Fx package/dist/index.js <<<"$main_listing" >/dev/null || fail "$MAIN_TARBALL lacks package/dist/index.js"
if grep -E '\.node$' <<<"$main_listing"; then
  fail "the thin main package must not carry a native binary"
fi

# --- Installed import from a fresh consumer outside the source tree ---------
# The generic linux-arm64-gnu package is not installed here, so a successful
# import proves the loader selected the Tegra package on this host. CUDA
# library search paths are removed so the import cannot lean on build-time
# locations.
CONSUMER="$STAGING_ROOT/consumer"
mkdir -p "$CONSUMER"
printf '{"name":"fathomdb-tegra-build-consumer","private":true,"type":"module"}\n' >"$CONSUMER/package.json"
(cd "$CONSUMER" && npm install --offline --ignore-scripts --no-audit --no-fund \
  "$MAIN_TARBALL" "$TEGRA_TARBALL" >/dev/null)
if ! (cd "$CONSUMER" && env -u LD_LIBRARY_PATH -u LIBRARY_PATH \
  node --input-type=module -e 'await import("fathomdb");'); then
  fail 'the installed Tegra package must import without CUDA library search paths'
fi
printf 'build-napi-cuda-tegra: installed packages import without CUDA library search paths\n'

mkdir -p "$OUT_DIR/native"
cp "$NATIVE_BINARY" "$OUT_DIR/native/"
(cd "$OUT_DIR" && sha256sum -- "native/fathomdb.$CUDA_TEGRA_NAPI_TRIPLE.node" \
  "$(basename "$MAIN_TARBALL")" "$(basename "$TEGRA_TARBALL")" >SHA256SUMS)
cat "$OUT_DIR/SHA256SUMS"
printf 'build-napi-cuda-tegra: main tarball %s\n' "$MAIN_TARBALL"
printf 'build-napi-cuda-tegra: tegra tarball %s\n' "$TEGRA_TARBALL"
