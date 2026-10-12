#!/usr/bin/env bash
# scripts/gcp-lab/remote-fork-cell.sh — runs ON a GPU lab VM created by
# scripts/gcp-cuda-lab.sh, after a driver is installed. Builds and tests one
# fork (coreyt/cudarc or coreyt/candle-fathomdb) against one CUDA toolkit and
# writes ~/fdb-lab-receipts/cells/<cell>.json whether or not the cell passes.
#
#   --cell <id> --fork cudarc|candle --ref <branch> --toolkit 12.6|13.0
#   --cuda-version 12060|13000 [--test-filter <substring>]
#
# The toolkit comes from NVIDIA's Ubuntu 24.04 apt repository, toolkit
# packages only (never a driver), into /usr/local/cuda-<ver>. Each cell
# selects its toolkit explicitly (CUDA_PATH, PATH, LD_LIBRARY_PATH with
# exactly one CUDA library directory) and records which CUDA libraries the
# tests actually loaded, because the loader cache can serve another
# toolkit's libraries (dev/notes/cuda-multiple-installations.md).
set -euo pipefail

CELL="" FORK="" REF="" TOOLKIT="" CUDA_VERSION="" TEST_FILTER=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --cell) CELL="$2"; shift 2 ;;
    --fork) FORK="$2"; shift 2 ;;
    --ref) REF="$2"; shift 2 ;;
    --toolkit) TOOLKIT="$2"; shift 2 ;;
    --cuda-version) CUDA_VERSION="$2"; shift 2 ;;
    --test-filter) TEST_FILTER="$2"; shift 2 ;;
    *) printf 'remote-fork-cell: unknown option %s\n' "$1" >&2; exit 2 ;;
  esac
done
die() { printf 'remote-fork-cell: %s\n' "$*" >&2; exit 2; }
[[ "$CELL" =~ ^[a-z0-9-]{1,40}$ ]] || die "--cell must match [a-z0-9-]{1,40}"
[ -n "$REF" ] || die "--ref is required"
case "$FORK" in
  cudarc) REPO_URL="https://github.com/coreyt/cudarc.git" ;;
  candle) REPO_URL="https://github.com/coreyt/candle-fathomdb.git" ;;
  *) die "--fork must be cudarc or candle" ;;
esac
case "$TOOLKIT" in
  12.6) expected_version=12060 ;;
  13.0) expected_version=13000 ;;
  *) die "--toolkit must be 12.6 or 13.0" ;;
esac
[ "$CUDA_VERSION" = "$expected_version" ] || die "--cuda-version $CUDA_VERSION does not match toolkit $TOOLKIT ($expected_version)"

OUT="$HOME/fdb-lab-receipts"
mkdir -p "$OUT/cells"
LOG="$OUT/cells/$CELL.log"
exec >>"$LOG" 2>&1
started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
driver_before="$(nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>&1 || true)"

export DEBIAN_FRONTEND=noninteractive
CUDA_HOME_DIR="/usr/local/cuda-$TOOLKIT"
if [ ! -x "$CUDA_HOME_DIR/bin/nvcc" ]; then
  if [ ! -e /usr/share/keyrings/cuda-archive-keyring.gpg ]; then
    keyring_dir="$(mktemp -d)"
    curl -fsSL -o "$keyring_dir/cuda-keyring.deb" \
      https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb
    sudo dpkg -i "$keyring_dir/cuda-keyring.deb"
  fi
  sudo apt-get update -q
  dash="${TOOLKIT/./-}"
  # Toolkit pieces only: no cuda/cuda-drivers meta-package, so no driver.
  sudo apt-get install -yq --no-install-recommends build-essential pkg-config git curl \
    "cuda-nvcc-$dash" "cuda-cudart-dev-$dash" "cuda-nvrtc-dev-$dash" "cuda-cccl-$dash" \
    "libcublas-dev-$dash" "libcurand-dev-$dash"
fi
toolkit_nvcc="$("$CUDA_HOME_DIR/bin/nvcc" --version | tail -n 1)"

if [ ! -x "$HOME/.cargo/bin/rustup" ]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
fi
# shellcheck source=/dev/null
. "$HOME/.cargo/env"

SRC="$HOME/forks/$FORK"
if [ ! -d "$SRC/.git" ]; then
  git clone --quiet --branch "$REF" "$REPO_URL" "$SRC"
fi
cd "$SRC"
git fetch --quiet origin "$REF"
git checkout --quiet --detach FETCH_HEAD
sha="$(git rev-parse HEAD)"
toolchain="$(rustc --version)"

export CUDA_PATH="$CUDA_HOME_DIR" CUDA_ROOT="$CUDA_HOME_DIR" CUDARC_CUDA_VERSION="$CUDA_VERSION"
export PATH="$CUDA_HOME_DIR/bin:$PATH" LD_LIBRARY_PATH="$CUDA_HOME_DIR/lib64"
export CUDA_COMPUTE_CAP=75 CUDA_VISIBLE_DEVICES=0
# Each toolkit gets its own target dir so a cell never reuses another
# toolkit's kernels or build-script output.
export CARGO_TARGET_DIR="$HOME/target/$FORK-$TOOLKIT"

case "$FORK" in
  cudarc)
    build_cmd=(cargo build --lib -F "cuda-$CUDA_VERSION")
    test_args=(--lib -F "cuda-$CUDA_VERSION")
    tree_args=() ;;
  candle)
    build_cmd=(cargo build -p candle-core-fathomdb --features cuda)
    test_args=(-p candle-core-fathomdb --features cuda)
    tree_args=(-p candle-core-fathomdb --features cuda) ;;
esac
test_cmd=(cargo test "${test_args[@]}" -- --test-threads=1)
[ -z "$TEST_FILTER" ] || test_cmd+=("$TEST_FILTER")

build_start="$(date +%s)"
build_status=0
"${build_cmd[@]}" || build_status=$?
build_seconds=$(( $(date +%s) - build_start ))

test_status=-1 test_seconds=0 tests_passed=0 tests_failed=0 tests_ignored=0
TEST_OUT="$OUT/cells/$CELL.test.txt"
LD_OUT="$OUT/cells/$CELL.ld-files.txt"
if [ "$build_status" -eq 0 ]; then
  # Compile the tests first, so LD_DEBUG records only the test processes.
  cargo test "${test_args[@]}" --no-run || true
  test_start="$(date +%s)"
  test_status=0
  LD_DEBUG=files LD_DEBUG_OUTPUT="$OUT/cells/$CELL.ld" "${test_cmd[@]}" >"$TEST_OUT" 2>&1 || test_status=$?
  test_seconds=$(( $(date +%s) - test_start ))
  cat "$TEST_OUT"
  while IFS=, read -r p f i; do
    tests_passed=$((tests_passed + p)) tests_failed=$((tests_failed + f)) tests_ignored=$((tests_ignored + i))
  done < <(sed -nE 's/.*test result: [a-zA-Z]+\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored.*/\1,\2,\3/p' "$TEST_OUT")
  # The CUDA libraries the test processes opened, deduplicated.
  cat "$OUT/cells/$CELL.ld".* 2>/dev/null \
    | grep -oE 'file=[^ ]*(libcuda|libcudart|libcublas|libcublasLt|libnvrtc|libcurand)[^ ]*' \
    | sort -u >"$LD_OUT" || true
  rm -f "$OUT/cells/$CELL.ld".*
fi

lock_sha="$(sha256sum Cargo.lock 2>/dev/null | cut -d' ' -f1 || echo none)"
cudarc_resolved="$(cargo tree "${tree_args[@]}" -e normal -i cudarc --depth 0 2>/dev/null | head -n 1 || echo unknown)"
driver_after="$(nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>&1 || true)"
gpu="$(nvidia-smi --query-gpu=name,compute_cap --format=csv,noheader 2>&1 || true)"
kernel="$(uname -r)"
nproc_count="$(nproc)"
# shellcheck source=/dev/null
os_name="$(. /etc/os-release && printf '%s' "$PRETTY_NAME")"

CELL_ID="$CELL" CELL_FORK="$FORK" CELL_REPO="$REPO_URL" CELL_REF="$REF" CELL_SHA="$sha" \
CELL_TOOLKIT="$TOOLKIT" CELL_TOOLKIT_NVCC="$toolkit_nvcc" CELL_CUDA_VERSION="$CUDA_VERSION" \
CELL_TOOLCHAIN="$toolchain" CELL_BUILD_CMD="${build_cmd[*]}" CELL_BUILD_STATUS="$build_status" \
CELL_BUILD_SECONDS="$build_seconds" CELL_TEST_CMD="${test_cmd[*]}" CELL_TEST_STATUS="$test_status" \
CELL_TEST_SECONDS="$test_seconds" CELL_TESTS_PASSED="$tests_passed" CELL_TESTS_FAILED="$tests_failed" \
CELL_TESTS_IGNORED="$tests_ignored" CELL_DRIVER_BEFORE="$driver_before" CELL_DRIVER_AFTER="$driver_after" \
CELL_GPU="$gpu" CELL_KERNEL="$kernel" CELL_OS="$os_name" CELL_NPROC="$nproc_count" \
CELL_LOCK_SHA256="$lock_sha" CELL_CUDARC_RESOLVED="$cudarc_resolved" CELL_STARTED="$started" \
CELL_LD_LIBRARY_PATH="$LD_LIBRARY_PATH" \
python3 -I - "$OUT/cells/$CELL.json" "$LD_OUT" <<'PY'
import datetime, json, os, sys
fields = {k[len("CELL_"):].lower(): v for k, v in os.environ.items() if k.startswith("CELL_")}
for key in ("build_status", "build_seconds", "test_status", "test_seconds", "tests_passed",
            "tests_failed", "tests_ignored", "nproc"):
    fields[key] = int(fields[key])
try:
    with open(sys.argv[2]) as fh:
        fields["loaded_cuda_libraries"] = [line.strip()[len("file="):] for line in fh if line.strip()]
except FileNotFoundError:
    fields["loaded_cuda_libraries"] = []
fields["finished"] = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
with open(sys.argv[1], "w") as fh:
    json.dump(dict(sorted(fields.items())), fh, indent=2)
    fh.write("\n")
PY
cat "$OUT/cells/$CELL.json"
[ "$build_status" -eq 0 ] && [ "$test_status" -eq 0 ] && [ "$driver_before" = "$driver_after" ]
