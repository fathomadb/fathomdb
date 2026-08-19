#!/usr/bin/env bash
# Build the Jetson/Tegra CUDA Python wheel host-natively from the release
# contract (0.8.23 Slice 80.6, D-80.6-1/2/4).
#
# Scope, deliberately narrow (D-80.6-2): the Python wheel ONLY. Not the npm
# N-API binding, not the CLI tarball. Python is the primary SDK surface and the
# one the x86_64 CUDA lane already proves most thoroughly; a second consumer can
# be added later without redoing any of this sub-slice's proof.
#
# Host-native, not containerized (§ 7 80.6 D-1): the Tegra CUDA runtime is
# host-bound, `sbsa-linux` ships no SASS/PTX slices for Tegra iGPUs, and no
# manylinux image carries a Tegra CUDA toolkit. The resulting wheel therefore
# carries JetPack's glibc floor (2.35 — see scripts/release/glibc-floor-contract.sh)
# and the native `linux` platform tag rather than a manylinux one.
#
# This wrapper BUILDS and PROVES. It never uploads a wheel to any registry, and
# nothing in Slice 80.6 authorizes doing so: § 3 rules a Tegra artifact under
# the existing package names out of scope, and § 11 item 8 holds registry
# publication behind the staged gate.
#
# Usage: build-python-cuda-tegra.sh [--out <dir>] [--interpreter <python>] [--assert-only]
#
#   --assert-only  Run every toolchain-identity and link-environment assertion,
#                  print the resolved build environment, and exit 0 without
#                  compiling. The real build is long; this arm makes the part
#                  that can silently drift — the contract-to-host binding —
#                  cheap to verify on its own.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"
# shellcheck source=glibc-floor-contract.sh
. "$SCRIPT_DIR/glibc-floor-contract.sh"

OUT_DIR="$REPO_ROOT/target/tegra-python-dist"
INTERPRETER='python3'
ASSERT_ONLY=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      [ "$#" -ge 2 ] || { printf 'build-python-cuda-tegra: --out needs a directory\n' >&2; exit 2; }
      OUT_DIR="$2"
      shift 2
      ;;
    --interpreter)
      [ "$#" -ge 2 ] || { printf 'build-python-cuda-tegra: --interpreter needs a python\n' >&2; exit 2; }
      INTERPRETER="$2"
      shift 2
      ;;
    --assert-only)
      ASSERT_ONLY=1
      shift
      ;;
    *)
      printf 'usage: %s [--out <dir>] [--interpreter <python>] [--assert-only]\n' "$(basename "$0")" >&2
      exit 2
      ;;
  esac
done

fail() {
  printf 'build-python-cuda-tegra: %s\n' "$1" >&2
  exit 1
}

# --- Toolchain identity, asserted before anything is built ------------------
# Same discipline as build-napi-cuda.sh: the contract states what this host
# must be, and the build refuses to start on a host that is something else.

HOST_ARCH="$(uname -m)"
if [ "$HOST_ARCH" != "$CUDA_TEGRA_HOST_ARCH" ]; then
  fail "the Tegra wheel is built host-natively on $CUDA_TEGRA_HOST_ARCH; this host is $HOST_ARCH"
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

# The CUDA runtime is a normal toolkit dependency of the generated Candle
# kernels. On this host it lives under targets/aarch64-linux/lib (lib64 is a
# symlink to it), and WITHOUT LIBRARY_PATH naming it the link fails
# `cannot find -lcudart` — measured on this Orin, not predicted (design § 2.7).
if [ ! -f "$CUDA_TEGRA_HOST_CUDART_LIB/libcudart.so" ]; then
  fail "expected the CUDA runtime import library at $CUDA_TEGRA_HOST_CUDART_LIB/libcudart.so"
fi
# The NVIDIA driver library is loaded dynamically by the pinned Candle fork
# (never linked), and on L4T it lives in an arch-specific `nvidia/`
# subdirectory — the bare multiarch path carries only a `.so`, no `.so.1`.
# Assert it here so a driverless host fails at build time with this message
# rather than at first device use with a loader error.
if [ ! -e "$CUDA_TEGRA_HOST_DRIVER_LIB" ]; then
  fail "expected the L4T NVIDIA driver library at $CUDA_TEGRA_HOST_DRIVER_LIB"
fi

if ! command -v maturin >/dev/null 2>&1; then
  fail 'maturin is not on PATH'
fi
if ! maturin --version | grep -F "maturin $CUDA_MATURIN_VERSION"; then
  fail 'maturin does not match the release contract'
fi
if ! command -v "$INTERPRETER" >/dev/null 2>&1; then
  fail "interpreter $INTERPRETER is not on PATH"
fi

# --- Build environment, every value from the contract -----------------------
export CUDA_PATH="$CUDA_TEGRA_HOST_TOOLKIT_ROOT"
export CUDACXX="$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc"
export CC="$CUDA_HOST_CC_TEGRA_ORIN"
export CXX="$CUDA_HOST_CXX_TEGRA_ORIN"
export CUDAHOSTCXX="$CUDA_HOST_CXX_TEGRA_ORIN"
export NVCC_CCBIN="$CUDA_HOST_CXX_TEGRA_ORIN"
# Slice 80.4 made compute capability a per-target axis precisely so this script
# could select Tegra's without touching x86_64's. Orin is SM_87, and Tegra is
# not forward-compatible the way discrete GPUs are, so this is a single pin.
export CUDA_COMPUTE_CAP="$CUDA_COMPUTE_CAP_TEGRA_ORIN"
export LIBRARY_PATH="$CUDA_TEGRA_HOST_CUDART_LIB${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LD_LIBRARY_PATH="$CUDA_TEGRA_HOST_CUDART_LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin:$PATH"

printf 'build-python-cuda-tegra: CUDA_PATH=%s\n' "$CUDA_PATH"
printf 'build-python-cuda-tegra: CC=%s CXX=%s\n' "$CC" "$CXX"
printf 'build-python-cuda-tegra: CUDA_COMPUTE_CAP=%s\n' "$CUDA_COMPUTE_CAP"
printf 'build-python-cuda-tegra: LIBRARY_PATH=%s\n' "$LIBRARY_PATH"
printf 'build-python-cuda-tegra: LD_LIBRARY_PATH=%s\n' "$LD_LIBRARY_PATH"
printf 'build-python-cuda-tegra: features=%s\n' "$CUDA_PYTHON_FEATURES"

if [ "$ASSERT_ONLY" -eq 1 ]; then
  printf 'build-python-cuda-tegra: assertions passed; --assert-only, not building\n'
  exit 0
fi

mkdir -p "$OUT_DIR"
SOURCE_PYTHON="$REPO_ROOT/src/python"
SOURCE_PYPROJECT="$SOURCE_PYTHON/pyproject.toml"
BASE_VERSION="$($INTERPRETER - "$SOURCE_PYPROJECT" <<'PY'
from pathlib import Path
import re
import sys

contents = Path(sys.argv[1]).read_text(encoding="utf-8")
matches = re.findall(r'^version = "([^"]+)"$', contents, flags=re.MULTILINE)
if len(matches) != 1 or not re.fullmatch(r'[0-9]+(?:\.[0-9]+)+(?:[A-Za-z0-9.\-]*)?', matches[0]):
    raise SystemExit("pyproject.toml must contain exactly one validated PEP 440 base version")
print(matches[0])
PY
)"
TEGRA_LOCAL_VERSION="${BASE_VERSION}+tegra"
STAGING_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-tegra-python.XXXXXX")"
trap 'rm -rf "$STAGING_ROOT"' EXIT
STAGED_PYTHON="$STAGING_ROOT/python"
cp -a "$SOURCE_PYTHON" "$STAGED_PYTHON"
"$INTERPRETER" - "$STAGED_PYTHON/pyproject.toml" "$REPO_ROOT/src/rust/crates/fathomdb-py/Cargo.toml" "$TEGRA_LOCAL_VERSION" <<'PY'
from pathlib import Path
import re
import sys

pyproject = Path(sys.argv[1])
manifest = Path(sys.argv[2])
version = sys.argv[3]
contents = pyproject.read_text(encoding="utf-8")
contents, version_count = re.subn(
    r'^version = "[^"]+"$', f'version = "{version}"', contents, count=1, flags=re.MULTILINE
)
contents, manifest_count = re.subn(
    r'^manifest-path = "[^"]+"$', f'manifest-path = "{manifest}"', contents, count=1, flags=re.MULTILINE
)
if version_count != 1 or manifest_count != 1:
    raise SystemExit("staged pyproject.toml is missing its version or maturin manifest path")
pyproject.write_text(contents, encoding="utf-8")
PY
cd "$STAGED_PYTHON"
# `--compatibility linux` and `--auditwheel skip` are the honest tags for a
# host-bound artifact: it is NOT manylinux, and claiming otherwise would be the
# exact drift AC80-9's doc-truth gate exists to stop. D-80.6-1 keeps this wheel
# out of every registry, which is what makes an unauditable tag acceptable.
maturin build --release --out "$OUT_DIR" \
  --features "$CUDA_PYTHON_FEATURES" \
  --compatibility "$CUDA_TEGRA_WHEEL_COMPATIBILITY" \
  --auditwheel skip \
  --interpreter "$INTERPRETER"

mapfile -t WHEELS < <(find "$OUT_DIR" -maxdepth 1 -type f -name "fathomdb-${TEGRA_LOCAL_VERSION}-*.whl" -print)
if [ "${#WHEELS[@]}" -ne 1 ]; then
  fail "the Tegra CUDA build must produce exactly one fathomdb-${TEGRA_LOCAL_VERSION}-*.whl in $OUT_DIR"
fi
WHEEL="${WHEELS[0]}"
if ! unzip -p "$WHEEL" '*/METADATA' | grep -Fx "Version: ${TEGRA_LOCAL_VERSION}" >/dev/null; then
  fail "the Tegra CUDA wheel METADATA must contain Version: ${TEGRA_LOCAL_VERSION}"
fi
printf 'build-python-cuda-tegra: built %s\n' "$WHEEL"

# D-80.6-5 / AC80-26: this artifact's floor is HIGHER than the manylinux
# families' because it is host-bound — which is a declared floor, not an
# exemption. Assert it against the extension this build just produced, with
# the same gate every other family runs. check-glibc-floor.sh already takes an
# arbitrary `--floor X.Y`; only the choice of which floor to pass is new.
TEGRA_FLOOR="$(glibc_floor_for_family tegra)"
UNPACKED="$OUT_DIR/unpacked"
rm -rf "${UNPACKED:?}"
mkdir -p "$UNPACKED"
unzip -q "$WHEEL" -d "$UNPACKED"
EXTENSION="$(find "$UNPACKED" -type f -name '*.so' -print -quit)"
if [ -z "$EXTENSION" ]; then
  fail "the Tegra CUDA wheel contains no Python extension"
fi
bash "$REPO_ROOT/scripts/check-glibc-floor.sh" --floor "$TEGRA_FLOOR" "$EXTENSION"
printf 'build-python-cuda-tegra: %s is within the declared tegra glibc floor %s\n' \
  "$(basename "$EXTENSION")" "$TEGRA_FLOOR"
# The abstract form is `python -m pip install $WHEEL`; render the concrete
# wheel argument with Bash's reversible shell quoting for `--out` paths that
# contain spaces or shell metacharacters.
printf '%s' 'python -m pip install '
printf '%q' "$WHEEL"
printf '\n'
