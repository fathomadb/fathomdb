#!/usr/bin/env bash
# Forced CUDA must survive a large JavaScript heap when fathomdb is imported
# first (Jetson, Node CUDA addon).
#
# On the Jetson AGX Orin 64 GB (L4T R36, CUDA 12.6) cuInit needs an unmapped
# hole of at least 4 GiB in [8 GiB, 128 GiB) of the process address space, and
# V8 scatters 256 KiB heap pages through that window as the heap grows. With
# about a million live objects cuInit returned CUDA_ERROR_OUT_OF_MEMORY in 45 of
# 45 measured processes, so forced cuda:0 refused with cuda_probe_failed. The
# Node addon on aarch64 Linux initialises the CUDA driver when it is loaded, so
# an application that imports fathomdb before growing its heap keeps CUDA.
#
# Each run is a fresh Node process (tegra_node_early_cuinit_consumer.mjs):
# import the package, grow the heap to HEAP_OBJECTS objects, open with the
# default embedder under FATHOMDB_EMBED_DEVICE=cuda:0, embed. Every run must
# pass.
#
# Off Linux aarch64 it always prints SKIP: a platform exclusion. On Linux
# aarch64, each prerequisite below is a SKIP with its reason when missing (a
# FAIL under FATHOMDB_REQUIRE_LIVE=1):
#   - `node` on PATH;
#   - FATHOMDB_TEGRA_NODE_PACKAGE: a package root holding dist/index.js whose
#     native addon was built with embed-cuda (in-tree src/ts after a CUDA
#     `napi build`, or an installed node_modules/fathomdb). The ordinary
#     agent-test loop builds only a CPU debug addon, so it skips here;
#   - cuda:0 is an integrated GPU with 60-64 GiB of device memory (the
#     measured Jetson AGX Orin 64 GB). Another Jetson has a different driver
#     reservation size, so the heap size that defeats cuInit there is unknown.
# The default embedder's model must also be cached or downloadable. That is
# not checked up front: a run that cannot load it fails, never skips.
#
# Env: FATHOMDB_TEGRA_NODE_PACKAGE (required), FATHOMDB_TEGRA_EARLY_CUINIT_RUNS
# (default 3), FATHOMDB_TEGRA_EARLY_CUINIT_HEAP_OBJECTS (default 1000000).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONSUMER="$SCRIPT_DIR/tegra_node_early_cuinit_consumer.mjs"
RUNS="${FATHOMDB_TEGRA_EARLY_CUINIT_RUNS:-3}"
HEAP_OBJECTS="${FATHOMDB_TEGRA_EARLY_CUINIT_HEAP_OBJECTS:-1000000}"

skip() {
  if [ "${FATHOMDB_REQUIRE_LIVE:-}" = "1" ]; then
    printf 'FAIL  FATHOMDB_REQUIRE_LIVE=1 and a live prerequisite is missing: %s\n' "$1" >&2
    exit 1
  fi
  printf 'SKIP  tegra-node-early-cuinit: %s\n' "$1"
  exit 0
}

host_os="$(uname -s)"
host_arch="$(uname -m)"
if [ "$host_os" != "Linux" ] || [ "$host_arch" != "aarch64" ]; then
  # A platform exclusion, like a Rust target's file-level cfg: never a missing
  # live prerequisite, so FATHOMDB_REQUIRE_LIVE does not turn it into a FAIL.
  printf 'SKIP  tegra-node-early-cuinit: Linux aarch64 only; the early CUDA initialisation is compiled only there\n'
  exit 0
fi
command -v node >/dev/null 2>&1 || skip "node is not on PATH"
package="${FATHOMDB_TEGRA_NODE_PACKAGE:-}"
[ -n "$package" ] || skip "FATHOMDB_TEGRA_NODE_PACKAGE is not set (no CUDA-capable Node package to test)"
[ -f "$package/dist/index.js" ] || skip "no dist/index.js under FATHOMDB_TEGRA_NODE_PACKAGE=$package"

# Identify cuda:0 in a separate process so this probe cannot shape the address
# space of the processes under test.
target="$(python3 - <<'PY'
import ctypes

try:
    cuda = ctypes.CDLL("libcuda.so.1")
except OSError:
    print("no CUDA driver library")
    raise SystemExit
result = cuda.cuInit(0)
if result != 0:
    print(f"cuInit returned {result} in an unobstructed probe process")
    raise SystemExit
count = ctypes.c_int(0)
device = ctypes.c_int(0)
if cuda.cuDeviceGetCount(ctypes.byref(count)) != 0 or count.value < 1:
    print("no CUDA device")
    raise SystemExit
if cuda.cuDeviceGet(ctypes.byref(device), 0) != 0:
    print("cuDeviceGet(0) failed")
    raise SystemExit
integrated = ctypes.c_int(0)
CU_DEVICE_ATTRIBUTE_INTEGRATED = 18
if cuda.cuDeviceGetAttribute(ctypes.byref(integrated), CU_DEVICE_ATTRIBUTE_INTEGRATED, device) != 0:
    print("could not read CU_DEVICE_ATTRIBUTE_INTEGRATED")
    raise SystemExit
total = ctypes.c_size_t(0)
if cuda.cuDeviceTotalMem_v2(ctypes.byref(total), device) != 0:
    print("could not read device memory")
    raise SystemExit
gib = 1 << 30
if not integrated.value:
    print("cuda:0 is a discrete GPU; the heap-size failure is only established on the Jetson AGX Orin 64 GB")
elif not 60 * gib <= total.value < 64 * gib:
    print(f"cuda:0 is an integrated GPU with {total.value} B; only the Jetson AGX Orin 64 GB (65879896064 B) was measured")
else:
    print("target")
PY
)"
[ "$target" = "target" ] || skip "$target"

scratch="$(mktemp -d)"
cleanup() {
  case "$scratch" in
    "${TMPDIR:-/tmp}"/*|/tmp/*) rm -rf "$scratch" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$scratch" >&2 ;;
  esac
}
trap cleanup EXIT

failed=0
for run in $(seq 1 "$RUNS"); do
  set +e
  line="$(env FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_TEGRA_NODE_PACKAGE="$package" \
    HEAP_OBJECTS="$HEAP_OBJECTS" DB_DIR="$scratch" node "$CONSUMER" 2>"$scratch/stderr-$run")"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ] && grep -q '"outcome":"pass"' <<<"$line"; then
    printf 'PASS  run %s/%s: import first, %s heap objects, forced cuda:0 open and embed\n' \
      "$run" "$RUNS" "$HEAP_OBJECTS"
  elif grep -q '"kind":"cuda_not_compiled"' <<<"$line"; then
    skip "the native addon under $package was built without CUDA"
  else
    printf 'FAIL  run %s/%s (exit %s): %s\n' "$run" "$RUNS" "$rc" "$line" >&2
    tail -n 5 "$scratch/stderr-$run" >&2 || true
    failed=$((failed + 1))
  fi
done
if [ "$failed" -ne 0 ]; then
  printf 'FAIL  tegra-node-early-cuinit: %s of %s runs failed\n' "$failed" "$RUNS" >&2
  exit 1
fi
