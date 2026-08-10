#!/usr/bin/env bash
# Collect fail-closed CUDA build/link/CPU-compatibility evidence before release.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  printf 'usage: %s <witness-directory>\n' "$0" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WITNESS_DIR="$1"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"
# shellcheck source=cuda-image-attestation.sh
. "$SCRIPT_DIR/cuda-image-attestation.sh"
DEFAULT_EMBEDDER_HF_HOME="${FATHOMDB_CUDA_PREFLIGHT_HF_HOME:-${HF_HOME:-$HOME/.cache/huggingface}}"
DEFAULT_EMBEDDER_SNAPSHOT="$DEFAULT_EMBEDDER_HF_HOME/hub/models--${CUDA_DEFAULT_EMBEDDER_HF_REPO//\//--}/snapshots/$CUDA_DEFAULT_EMBEDDER_HF_REVISION"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'cuda-preflight: required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

for command in nvidia-smi docker readelf ldd python3 unzip npm; do
  require_command "$command"
done
if [ ! -x "$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" ]; then
  printf 'cuda-preflight: expected N-API host CUDA compiler at %s/bin/nvcc\n' \
    "$CUDA_NAPI_HOST_TOOLKIT_ROOT" >&2
  exit 1
fi
if ! "$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" --version | grep -F "$CUDA_NAPI_HOST_NVCC_VERSION"; then
  printf 'cuda-preflight: N-API host CUDA compiler does not match the release contract\n' >&2
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  printf 'cuda-preflight: Docker must be available to prove the driverless CPU container\n' >&2
  exit 1
fi
if ! assert_cuda_manylinux_image; then
  printf 'cuda-preflight: provisioned CUDA/manylinux image is absent or does not match the release contract\n' >&2
  printf 'cuda-preflight: run bash scripts/release/provision-cuda-manylinux.sh on the designated runner before rerunning\n' >&2
  exit 1
fi
if ! {
  printf '%s  %s/config.json\n' "$CUDA_DEFAULT_EMBEDDER_CONFIG_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
  printf '%s  %s/tokenizer.json\n' "$CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
  printf '%s  %s/model.safetensors\n' "$CUDA_DEFAULT_EMBEDDER_MODEL_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
} | sha256sum --check --status; then
  printf 'cuda-preflight: pinned default-embedder cache is absent, incomplete, or has a digest mismatch: %s\n' \
    "$DEFAULT_EMBEDDER_SNAPSHOT" >&2
  printf 'cuda-preflight: run bash scripts/release/provision-cuda-manylinux.sh on the designated runner before rerunning\n' >&2
  exit 1
fi

if [ -e "$WITNESS_DIR" ]; then
  printf 'cuda-preflight: witness directory must be new: %s\n' "$WITNESS_DIR" >&2
  exit 1
fi
mkdir -p "$WITNESS_DIR/python-dist" "$WITNESS_DIR/python-unpacked"
{
  printf 'generated_at_utc='; date --utc --iso-8601=seconds
  printf 'repository_commit='; git -C "$REPO_ROOT" rev-parse HEAD
  printf 'napi_host_cuda_toolkit_root=%s\n' "$CUDA_NAPI_HOST_TOOLKIT_ROOT"
  printf 'napi_host_nvcc_version=%s\n' "$CUDA_NAPI_HOST_NVCC_VERSION"
  printf 'cuda_manylinux=%s\n' "$CUDA_MANYLINUX"
  printf 'cuda_compute_cap=%s\n' "$CUDA_COMPUTE_CAP"
  printf 'cuda_napi_features=%s\n' "$CUDA_NAPI_FEATURES"
  printf 'cuda_python_features=%s\n' "$CUDA_PYTHON_FEATURES"
  printf 'cuda_manylinux_image=%s\n' "$CUDA_MANYLINUX_IMAGE"
  printf 'cuda_manylinux_python=%s\n' "$CUDA_MANYLINUX_PYTHON"
  printf 'driverless_python_image=%s\n' "$CUDA_DRIVERLESS_PYTHON_IMAGE"
  printf 'driverless_node_image=%s\n' "$CUDA_DRIVERLESS_NODE_IMAGE"
  printf 'default_embedder_hf_home=%s\n' "$DEFAULT_EMBEDDER_HF_HOME"
  printf 'default_embedder_snapshot=%s\n' "$DEFAULT_EMBEDDER_SNAPSHOT"
  printf '\n[nvidia-smi]\n'; nvidia-smi --query-gpu=index,name,driver_version --format=csv,noheader
  printf '\n[napi host nvcc]\n'; "$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" --version
  printf '\n[docker]\n'; docker version --format '{{.Server.Version}}'
  printf '\n[cuda-manylinux image]\n'; docker image inspect --format '{{.Id}} {{.RepoDigests}} {{json .Config.Labels}}' "$CUDA_MANYLINUX_IMAGE"
} | tee "$WITNESS_DIR/environment.txt"

printf 'cuda-preflight: build Linux CUDA N-API artifact\n'
"$SCRIPT_DIR/build-napi-cuda.sh"
NAPI_BINARY="$(find "$REPO_ROOT/src/ts" -maxdepth 1 -type f -name 'fathomdb.linux-x64-gnu.node' -print -quit)"
if [ -z "$NAPI_BINARY" ]; then
  printf 'cuda-preflight: Linux CUDA N-API build produced no linux-x64-gnu artifact\n' >&2
  exit 1
fi

printf 'cuda-preflight: build Linux CUDA Python wheel\n'
docker run --rm \
  --mount "type=bind,src=$REPO_ROOT,dst=/workspace" \
  --mount "type=bind,src=$WITNESS_DIR,dst=/witness" \
  --workdir /workspace/src/python \
  -e CUDA_PATH=/usr/local/cuda-12.6 \
  -e CUDACXX=/usr/local/cuda-12.6/bin/nvcc \
  -e CUDA_COMPUTE_CAP \
  -e CUDA_PYTHON_FEATURES \
  -e CUDA_MANYLINUX \
  -e CUDA_MANYLINUX_PYTHON \
  -e CUDA_RUST_VERSION \
  -e CUDA_MATURIN_VERSION \
  -e "CC=$CUDA_MANYLINUX_CC" \
  -e "CXX=$CUDA_MANYLINUX_CXX" \
  -e "CUDAHOSTCXX=$CUDA_MANYLINUX_CXX" \
  -e "NVCC_CCBIN=$CUDA_MANYLINUX_CXX" \
  -e CUDA_MANYLINUX_GCC_VERSION -e CUDA_MANYLINUX_CC -e CUDA_MANYLINUX_CXX \
  "$CUDA_MANYLINUX_IMAGE" \
  sh -ceu '
    command -v maturin
    command -v auditwheel
    maturin --version | grep -F "maturin $CUDA_MATURIN_VERSION"
    rustc --version | grep -F "rustc $CUDA_RUST_VERSION"
    test "$CC" = "$CUDA_MANYLINUX_CC"
    test "$CXX" = "$CUDA_MANYLINUX_CXX"
    test "$CUDAHOSTCXX" = "$CUDA_MANYLINUX_CXX"
    test "$NVCC_CCBIN" = "$CUDA_MANYLINUX_CXX"
    "$CC" --version | grep -F "$CUDA_MANYLINUX_GCC_VERSION"
    "$CXX" --version | grep -F "$CUDA_MANYLINUX_GCC_VERSION"
    /usr/local/cuda-12.6/bin/nvcc --version | grep -F "release 12.6"
    maturin build --release --out /witness/python-dist \
      --features "$CUDA_PYTHON_FEATURES" \
      --manylinux "$CUDA_MANYLINUX" \
      --interpreter "$CUDA_MANYLINUX_PYTHON"
  ' | tee "$WITNESS_DIR/manylinux-build.txt"
WHEEL="$(find "$WITNESS_DIR/python-dist" -maxdepth 1 -type f -name '*.whl' -print -quit)"
if [ -z "$WHEEL" ]; then
  printf 'cuda-preflight: CUDA Python build produced no wheel\n' >&2
  exit 1
fi
WHEEL_BASENAME="$(basename "$WHEEL")"
docker run --rm --network none \
  --mount "type=bind,src=$WITNESS_DIR,dst=/witness,readonly" \
  -e "WHEEL_BASENAME=$WHEEL_BASENAME" \
  "$CUDA_MANYLINUX_IMAGE" \
  sh -ceu 'auditwheel show "/witness/python-dist/$WHEEL_BASENAME"' \
  | tee "$WITNESS_DIR/python-auditwheel.txt"
if ! grep -F 'manylinux_2_28' "$WITNESS_DIR/python-auditwheel.txt" >/dev/null; then
  printf 'cuda-preflight: Python wheel is not reported as manylinux_2_28 by auditwheel\n' >&2
  exit 1
fi
unzip -q "$WHEEL" -d "$WITNESS_DIR/python-unpacked"
PYTHON_EXTENSION="$(find "$WITNESS_DIR/python-unpacked" -type f -name '*.so' -print -quit)"
if [ -z "$PYTHON_EXTENSION" ]; then
  printf 'cuda-preflight: CUDA wheel contains no Python extension\n' >&2
  exit 1
fi

{
  printf '[node readelf]\n'; readelf -d "$NAPI_BINARY"
  printf '\n[node ldd]\n'; ldd "$NAPI_BINARY"
  printf '\n[python readelf]\n'; readelf -d "$PYTHON_EXTENSION"
  printf '\n[python ldd]\n'; ldd "$PYTHON_EXTENSION"
} | tee "$WITNESS_DIR/dynamic-dependencies.txt"

printf 'cuda-preflight: stage installed N-API package for the driverless smoke\n'
(
  cd "$REPO_ROOT/src/ts"
  npm exec -- tsc -p tsconfig.build.json
)
NPM_STAGING="$WITNESS_DIR/npm-artifacts"
NPM_MAIN="$NPM_STAGING/main"
NPM_PLATFORM_ROOT="$NPM_STAGING/platforms"
NPM_PLATFORM="$NPM_PLATFORM_ROOT/linux-x64-gnu"
mkdir -p "$NPM_MAIN" "$NPM_PLATFORM"
cp "$REPO_ROOT/src/ts/package.json" "$REPO_ROOT/src/ts/LICENSE" "$NPM_MAIN/"
cp -R "$REPO_ROOT/src/ts/dist" "$NPM_MAIN/dist"
cp "$REPO_ROOT/src/ts/npm/linux-x64-gnu/package.json" "$REPO_ROOT/src/ts/npm/linux-x64-gnu/LICENSE" \
  "$NPM_PLATFORM/"
cp "$NAPI_BINARY" "$NPM_PLATFORM/fathomdb.linux-x64-gnu.node"
bash "$SCRIPT_DIR/npm-inject-optional-deps.sh" "$NPM_MAIN" "$NPM_PLATFORM_ROOT"
NPM_PLATFORM_TARBALL="$(cd "$NPM_PLATFORM" && npm pack --silent)"
NPM_MAIN_TARBALL="$(cd "$NPM_MAIN" && npm pack --silent)"

printf 'cuda-preflight: prove the installed Python wheel defaults to CPU in a driverless container\n'
docker run --rm --network none \
  --mount "type=bind,src=$WHEEL,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  -e HF_HOME=/fathomdb-hf \
  -e FATHOMDB_EMBED_DEVICE=cpu \
  "$CUDA_DRIVERLESS_PYTHON_IMAGE" \
  sh -ceu '
    test ! -e /dev/nvidiactl
    python -m pip install --no-deps /input/fathomdb.whl
    python - <<"PY"
import pathlib
import sys
import tempfile

from fathomdb import Engine

with tempfile.TemporaryDirectory() as directory:
    db_path = pathlib.Path(directory) / "driverless.fdb"
    engine = Engine.open(str(db_path), use_default_embedder=True)
    assert len(engine.embed("driverless Python CUDA-capable default-embedder proof")) == 384
    engine.write([{"kind": "doc", "body": "{}", "source_id": "smoke:cuda-driverless"}])
    engine.search("smoke")
    engine.close()
print("driverless installed Python CUDA-capable default-embedder CPU smoke: ok")
PY
  ' | tee "$WITNESS_DIR/driverless-python-cpu-smoke.txt"

printf 'cuda-preflight: prove the installed N-API package defaults to CPU in a driverless container\n'
docker run --rm --network none \
  --mount "type=bind,src=$NPM_MAIN/$NPM_MAIN_TARBALL,dst=/input/fathomdb.tgz,readonly" \
  --mount "type=bind,src=$NPM_PLATFORM/$NPM_PLATFORM_TARBALL,dst=/input/fathomdb-linux-x64-gnu.tgz,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  -e HF_HOME=/fathomdb-hf \
  -e FATHOMDB_EMBED_DEVICE=cpu \
  "$CUDA_DRIVERLESS_NODE_IMAGE" \
  sh -ceu '
    test ! -e /dev/nvidiactl
    mkdir /consumer
    cd /consumer
    cat > package.json <<"JSON"
{
  "private": true,
  "type": "module",
  "dependencies": {
    "fathomdb": "file:/input/fathomdb.tgz",
    "fathomdb-linux-x64-gnu": "file:/input/fathomdb-linux-x64-gnu.tgz"
  }
}
JSON
    npm install --offline --ignore-scripts --no-audit --no-fund
    node --input-type=module - <<"JS"
import { Engine } from "fathomdb";

const engine = await Engine.open("/tmp/driverless-node.fdb", { useDefaultEmbedder: true });
const vector = await engine.embed("driverless N-API CUDA-capable default-embedder proof");
if (vector.length !== 384) throw new Error("expected 384-vector, got " + vector.length);
await engine.write([{ kind: "doc", body: "{}", sourceId: "smoke:cuda-driverless-napi" }]);
await engine.search("smoke");
await engine.close();
console.log("driverless installed N-API CUDA-capable default-embedder CPU smoke: ok");
JS
  ' | tee "$WITNESS_DIR/driverless-napi-cpu-smoke.txt"

wait_for_cuda_zero_container_pid() {
  local container="$1" label="$2" witness="$3" pid compute_pids exit_code
  for _ in $(seq 1 30); do
    if ! docker inspect --format '{{.State.Running}}' "$container" | grep -Fx true >/dev/null; then
      printf 'cuda-preflight: %s GPU smoke exited before its CUDA:0 PID was observed\n' "$label" >&2
      docker logs "$container" >&2 || true
      docker rm "$container" >/dev/null 2>&1 || true
      exit 1
    fi
    pid="$(docker inspect --format '{{.State.Pid}}' "$container")"
    compute_pids="$(nvidia-smi --id=0 --query-compute-apps=pid --format=csv,noheader)"
    if printf '%s\n' "$compute_pids" | grep -Fx "$pid" >/dev/null; then
      {
        printf 'container=%s\n' "$container"
        printf 'host_pid=%s\n' "$pid"
        printf 'cuda_device=0\n'
        nvidia-smi --id=0 --query-compute-apps=pid,process_name,used_memory --format=csv,noheader
      } | tee "$witness"
      exit_code="$(docker wait "$container")"
      docker logs "$container" | tee -a "$witness"
      docker rm "$container" >/dev/null
      if [ "$exit_code" != 0 ]; then
        printf 'cuda-preflight: %s GPU smoke exited %s after CUDA:0 witness\n' "$label" "$exit_code" >&2
        exit 1
      fi
      return
    fi
    sleep 1
  done
  printf 'cuda-preflight: %s GPU smoke PID was not observed on CUDA:0\n' "$label" >&2
  docker logs "$container" >&2 || true
  docker rm "$container" >/dev/null 2>&1 || true
  exit 1
}

wait_for_cuda_zero_host_pid() {
  local pid="$1" label="$2" witness="$3" compute_pids
  for _ in $(seq 1 30); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      printf 'cuda-preflight: %s GPU smoke exited before its CUDA:0 PID was observed\n' "$label" >&2
      return 1
    fi
    compute_pids="$(nvidia-smi --id=0 --query-compute-apps=pid --format=csv,noheader)"
    if printf '%s\n' "$compute_pids" | grep -Fx "$pid" >/dev/null; then
      {
        printf 'host_pid=%s\n' "$pid"
        printf 'cuda_device=0\n'
        nvidia-smi --id=0 --query-compute-apps=pid,process_name,used_memory --format=csv,noheader
      } | tee "$witness"
      if ! wait "$pid"; then
        printf 'cuda-preflight: %s GPU smoke failed after CUDA:0 witness\n' "$label" >&2
        return 1
      fi
      return
    fi
    sleep 1
  done
  printf 'cuda-preflight: %s GPU smoke PID was not observed on CUDA:0\n' "$label" >&2
  return 1
}

PYTHON_GPU_SMOKE="$WITNESS_DIR/gpu-python-smoke.py"
cat > "$PYTHON_GPU_SMOKE" <<'PY'
import pathlib
import tempfile
import time

from fathomdb import Engine

with tempfile.TemporaryDirectory() as directory:
    gpu_db_path = pathlib.Path(directory) / "cuda-python.fdb"
    engine = Engine.open(str(gpu_db_path), use_default_embedder=True)
    assert len(engine.embed("installed Python CUDA artifact GPU proof")) == 384
    engine.write([{"kind": "doc", "body": "{}", "source_id": "smoke:cuda-python"}])
    engine.search("smoke")
    engine.close()
time.sleep(20)
PY

printf 'cuda-preflight: prove the installed Python wheel uses CUDA:0\n'
PYTHON_GPU_CONTAINER="$(docker run -d --gpus '"'"'device=0'"'"' --network none \
  --mount "type=bind,src=$WHEEL,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$PYTHON_GPU_SMOKE,dst=/input/gpu-python-smoke.py,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  -e HF_HOME=/fathomdb-hf \
  -e FATHOMDB_EMBED_DEVICE=cuda:0 \
  -e CUDA_MANYLINUX_PYTHON \
  "$CUDA_MANYLINUX_IMAGE" \
  sh -ceu '"$CUDA_MANYLINUX_PYTHON" -m pip install --no-deps /input/fathomdb.whl; exec "$CUDA_MANYLINUX_PYTHON" /input/gpu-python-smoke.py')"
wait_for_cuda_zero_container_pid "$PYTHON_GPU_CONTAINER" 'installed Python wheel' "$WITNESS_DIR/gpu-python-cuda-witness.txt"

NODE_GPU_SMOKE="$WITNESS_DIR/gpu-node-smoke.mjs"
cat > "$NODE_GPU_SMOKE" <<'JS'
import { Engine } from "fathomdb";

const gpuOpenOptions = {
  useDefaultEmbedder: true,
};
const engine = await Engine.open("/tmp/cuda-node.fdb", gpuOpenOptions);
const vector = await engine.embed("installed N-API CUDA artifact GPU proof");
if (vector.length !== 384) throw new Error("expected 384-vector, got " + vector.length);
await engine.write([{ kind: "doc", body: "{}", sourceId: "smoke:cuda-node" }]);
await engine.search("smoke");
await engine.close();
await new Promise((resolve) => setTimeout(resolve, 20_000));
JS

printf 'cuda-preflight: prove the installed N-API package uses CUDA:0\n'
NODE_GPU_CONSUMER="$WITNESS_DIR/gpu-node-consumer"
mkdir "$NODE_GPU_CONSUMER"
cat > "$NODE_GPU_CONSUMER/package.json" <<EOF
{
  "private": true,
  "type": "module",
  "dependencies": {
    "fathomdb": "file:$NPM_MAIN/$NPM_MAIN_TARBALL",
    "fathomdb-linux-x64-gnu": "file:$NPM_PLATFORM/$NPM_PLATFORM_TARBALL"
  }
}
EOF
(
  cd "$NODE_GPU_CONSUMER"
  npm install --offline --ignore-scripts --no-audit --no-fund
  HF_HOME="$DEFAULT_EMBEDDER_HF_HOME" FATHOMDB_EMBED_DEVICE=cuda:0 exec node "$NODE_GPU_SMOKE"
) > "$WITNESS_DIR/gpu-node-cuda-smoke.txt" 2>&1 &
NODE_GPU_PID="$!"
wait_for_cuda_zero_host_pid "$NODE_GPU_PID" 'installed N-API package' "$WITNESS_DIR/gpu-node-cuda-witness.txt"

printf 'cuda-preflight: pass; witness at %s\n' "$WITNESS_DIR"
