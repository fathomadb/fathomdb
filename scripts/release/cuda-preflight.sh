#!/usr/bin/env bash
# Collect fail-closed CUDA build/link/device evidence without publishing.
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  printf 'usage: %s <witness-directory> [--rerank-cuda]\n' "$0" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$1"
RERANK_CUDA=false
if [ "${2:-}" = '--rerank-cuda' ]; then
  RERANK_CUDA=true
elif [ "$#" -eq 2 ]; then
  printf 'cuda-preflight: unknown mode: %s\n' "$2" >&2
  exit 2
fi
CONTAINER_UID="$(id -u)"
CONTAINER_GID="$(id -g)"
CONTAINER_USER="$CONTAINER_UID:$CONTAINER_GID"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"
. "$SCRIPT_DIR/cuda-image-attestation.sh"
if [ "$RERANK_CUDA" = true ]; then
  CUDA_NAPI_FEATURES="$CUDA_RERANK_NAPI_FEATURES"
  CUDA_PYTHON_FEATURES="$CUDA_RERANK_PYTHON_FEATURES"
fi
DEFAULT_EMBEDDER_HF_HOME="${FATHOMDB_CUDA_PREFLIGHT_HF_HOME:-${HF_HOME:-$HOME/.cache/huggingface}}"
DEFAULT_EMBEDDER_SNAPSHOT="$DEFAULT_EMBEDDER_HF_HOME/hub/models--${CUDA_DEFAULT_EMBEDDER_HF_REPO//\//--}/snapshots/$CUDA_DEFAULT_EMBEDDER_HF_REVISION"
DEFAULT_RERANKER_CACHE_ROOT="${FATHOMDB_CUDA_PREFLIGHT_RERANKER_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}}"
DEFAULT_RERANKER_CACHE="$DEFAULT_RERANKER_CACHE_ROOT/fathomdb/reranker/0290849b0459"

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'cuda-preflight: required command not found: %s\n' "$1" >&2
    exit 1
  }
}

for command in nvidia-smi docker readelf ldd python3 unzip npm sha256sum; do
  require_command "$command"
done
if [ -e "$OUTPUT_DIR" ]; then
  printf 'cuda-preflight: witness directory must be new: %s\n' "$OUTPUT_DIR" >&2
  exit 1
fi
if [ ! -x "$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" ]; then
  printf 'cuda-preflight: expected N-API host CUDA compiler at %s/bin/nvcc\n' \
    "$CUDA_NAPI_HOST_TOOLKIT_ROOT" >&2
  exit 1
fi
if ! "$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" --version | grep -F "$CUDA_NAPI_HOST_NVCC_VERSION"; then
  printf 'cuda-preflight: N-API host CUDA compiler does not match the release contract\n' >&2
  exit 1
fi
docker info >/dev/null 2>&1 || {
  printf 'cuda-preflight: Docker must be available\n' >&2
  exit 1
}
assert_cuda_manylinux_image || {
  printf 'cuda-preflight: provisioned CUDA/manylinux image is absent or differs\n' >&2
  exit 1
}
if ! {
  printf '%s  %s/config.json\n' "$CUDA_DEFAULT_EMBEDDER_CONFIG_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
  printf '%s  %s/tokenizer.json\n' "$CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
  printf '%s  %s/model.safetensors\n' "$CUDA_DEFAULT_EMBEDDER_MODEL_SHA256" "$DEFAULT_EMBEDDER_SNAPSHOT"
} | sha256sum --check --status; then
  printf 'cuda-preflight: pinned default-embedder cache is absent or differs: %s\n' \
    "$DEFAULT_EMBEDDER_SNAPSHOT" >&2
  exit 1
fi
if [ "$RERANK_CUDA" = true ] && ! {
  printf '%s  %s/config.json\n' "$CUDA_RERANKER_CONFIG_SHA256" "$DEFAULT_RERANKER_CACHE"
  printf '%s  %s/tokenizer.json\n' "$CUDA_RERANKER_TOKENIZER_SHA256" "$DEFAULT_RERANKER_CACHE"
  printf '%s  %s/model.safetensors\n' "$CUDA_RERANKER_MODEL_SHA256" "$DEFAULT_RERANKER_CACHE"
} | sha256sum --check --status; then
  printf 'cuda-preflight: pinned TinyBERT reranker cache is absent or differs: %s\n' \
    "$DEFAULT_RERANKER_CACHE" >&2
  exit 1
fi
if [ "$RERANK_CUDA" = true ]; then
  [ -d "$DEFAULT_RERANKER_CACHE" ] && [ ! -L "$DEFAULT_RERANKER_CACHE" ] || {
    printf 'cuda-preflight: pinned TinyBERT reranker cache must be a non-symlink directory\n' >&2
    exit 1
  }
  for name in config.json tokenizer.json model.safetensors; do
    [ -f "$DEFAULT_RERANKER_CACHE/$name" ] && [ ! -L "$DEFAULT_RERANKER_CACHE/$name" ] || {
      printf 'cuda-preflight: pinned TinyBERT reranker cache member is absent or symlinked: %s\n' "$name" >&2
      exit 1
    }
  done
fi

WORK_ROOT="$(mktemp -d)"
trap 'rm -rf "$WORK_ROOT"' EXIT
WORK_DIR="$WORK_ROOT/work"
mkdir -p "$WORK_DIR/python-dist" "$WORK_DIR/python-unpacked" "$WORK_DIR/cache" "$WORK_DIR/tmp"

{
  printf 'generated_at_utc='; date --utc --iso-8601=seconds
  printf 'repository_commit='; git -C "$REPO_ROOT" rev-parse HEAD
  printf 'target=x86_64-unknown-linux-gnu\n'
  printf 'napi_host_cuda_toolkit_root=%s\n' "$CUDA_NAPI_HOST_TOOLKIT_ROOT"
  printf 'napi_host_nvcc_version=%s\n' "$CUDA_NAPI_HOST_NVCC_VERSION"
  printf 'cuda_manylinux=%s\n' "$CUDA_MANYLINUX"
  printf 'cuda_compute_cap=%s\n' "$CUDA_COMPUTE_CAP"
  printf 'cuda_napi_features=%s\n' "$CUDA_NAPI_FEATURES"
  printf 'cuda_python_features=%s\n' "$CUDA_PYTHON_FEATURES"
  printf 'rerank_cuda=%s\n' "$RERANK_CUDA"
  if [ "$RERANK_CUDA" = true ]; then
    printf 'reranker_cache_root=%s\n' "$DEFAULT_RERANKER_CACHE_ROOT"
  fi
  printf 'cuda_manylinux_image=%s\n' "$CUDA_MANYLINUX_IMAGE"
  printf 'cuda_manylinux_python=%s\n' "$CUDA_MANYLINUX_PYTHON"
  printf '\n[nvidia-smi]\n'; nvidia-smi --query-gpu=index,uuid,name,driver_version --format=csv,noheader
  printf '\n[napi host nvcc]\n'; "$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" --version
  printf '\n[docker]\n'; docker version --format '{{.Server.Version}}'
  printf '\n[cuda-manylinux image]\n'; docker image inspect --format '{{.Id}} {{.RepoDigests}} {{json .Config.Labels}}' "$CUDA_MANYLINUX_IMAGE"
} | tee "$WORK_DIR/environment.txt"

printf 'cuda-preflight: build Linux CUDA N-API artifact\n'
CC="$CUDA_NAPI_HOST_CC" CXX="$CUDA_NAPI_HOST_CXX" \
  CUDAHOSTCXX="$CUDA_NAPI_HOST_CXX" NVCC_CCBIN="$CUDA_NAPI_HOST_CXX" \
  "$SCRIPT_DIR/build-napi-cuda.sh"
NAPI_BINARY="$(find "$REPO_ROOT/src/ts" -maxdepth 1 -type f -name 'fathomdb.linux-x64-gnu.node' -print -quit)"
[ -n "$NAPI_BINARY" ] || {
  printf 'cuda-preflight: N-API build produced no linux-x64-gnu artifact\n' >&2
  exit 1
}

printf 'cuda-preflight: build Linux CUDA Python wheel\n'
docker run --rm \
  --user "$CONTAINER_USER" \
  --mount "type=bind,src=$REPO_ROOT,dst=/workspace,readonly" \
  --mount "type=bind,src=$WORK_DIR,dst=/witness" \
  --workdir /workspace/src/python \
  -e HOME=/tmp \
  -e CARGO_HOME=/tmp/fathomdb-cargo \
  -e RUSTUP_HOME=/opt/fathomdb/rustup \
  -e CUDA_RUSTUP_TOOLCHAIN \
  -e CARGO_TARGET_DIR=/tmp/fathomdb-cargo-target \
  -e "RUSTUP_TOOLCHAIN=$CUDA_RUSTUP_TOOLCHAIN" \
  -e CUDA_PATH=/usr/local/cuda-12.6 \
  -e CUDACXX=/usr/local/cuda-12.6/bin/nvcc \
  -e CUDA_COMPUTE_CAP -e CUDA_PYTHON_FEATURES -e CUDA_MANYLINUX -e CUDA_MANYLINUX_PYTHON \
  -e CUDA_RUST_VERSION -e CUDA_MATURIN_VERSION \
  -e "CC=$CUDA_MANYLINUX_CC" \
  -e "CXX=$CUDA_MANYLINUX_CXX" \
  -e "CUDAHOSTCXX=$CUDA_MANYLINUX_CXX" \
  -e "NVCC_CCBIN=$CUDA_MANYLINUX_CXX" \
  -e "LIBRARY_PATH=$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB" \
  -e "LD_LIBRARY_PATH=$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB" \
  -e CUDA_MANYLINUX_GCC_VERSION -e CUDA_MANYLINUX_CC -e CUDA_MANYLINUX_CXX \
  -e CUDA_MANYLINUX_CUDA_LIB64 -e CUDA_MANYLINUX_GCC_LIB \
  "$CUDA_MANYLINUX_IMAGE" \
  sh -ceu '
    command -v maturin
    command -v auditwheel
    maturin --version | grep -F "maturin $CUDA_MATURIN_VERSION"
    rustc --version | grep -F "rustc $CUDA_RUST_VERSION"
    test "$RUSTUP_TOOLCHAIN" = "$CUDA_RUSTUP_TOOLCHAIN"
    test ! -w /opt/fathomdb/rustup
    test "$CC" = "$CUDA_MANYLINUX_CC"
    test "$CXX" = "$CUDA_MANYLINUX_CXX"
    test "$CUDAHOSTCXX" = "$CUDA_MANYLINUX_CXX"
    test "$NVCC_CCBIN" = "$CUDA_MANYLINUX_CXX"
    test "$LIBRARY_PATH" = "$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB"
    test "$LD_LIBRARY_PATH" = "$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB"
    "$CC" --version | grep -F "$CUDA_MANYLINUX_GCC_VERSION"
    "$CXX" --version | grep -F "$CUDA_MANYLINUX_GCC_VERSION"
    maturin build --release --out /witness/python-dist \
      --features "$CUDA_PYTHON_FEATURES" \
      --manylinux "$CUDA_MANYLINUX" \
      --interpreter "$CUDA_MANYLINUX_PYTHON"
  ' | tee "$WORK_DIR/manylinux-build.txt"
WHEEL="$(find "$WORK_DIR/python-dist" -maxdepth 1 -type f -name '*.whl' -print -quit)"
[ -n "$WHEEL" ] || {
  printf 'cuda-preflight: CUDA Python build produced no wheel\n' >&2
  exit 1
}
WHEEL_BASENAME="$(basename "$WHEEL")"
docker run --rm --network none \
  --mount "type=bind,src=$WORK_DIR,dst=/witness,readonly" \
  -e "WHEEL_BASENAME=$WHEEL_BASENAME" \
  "$CUDA_MANYLINUX_IMAGE" \
  sh -ceu 'auditwheel show "/witness/python-dist/$WHEEL_BASENAME"' \
  | tee "$WORK_DIR/python-auditwheel.txt"
grep -F 'manylinux_2_28' "$WORK_DIR/python-auditwheel.txt" >/dev/null || {
  printf 'cuda-preflight: wheel is not manylinux_2_28\n' >&2
  exit 1
}
unzip -q "$WHEEL" -d "$WORK_DIR/python-unpacked"
PYTHON_EXTENSION="$(find "$WORK_DIR/python-unpacked" -type f -name '*.so' -print -quit)"
[ -n "$PYTHON_EXTENSION" ] || {
  printf 'cuda-preflight: wheel contains no Python extension\n' >&2
  exit 1
}
{
  printf '[node readelf]\n'; readelf -d "$NAPI_BINARY"
  printf '\n[node ldd]\n'; ldd "$NAPI_BINARY"
  printf '\n[python readelf]\n'; readelf -d "$PYTHON_EXTENSION"
  printf '\n[python ldd]\n'; ldd "$PYTHON_EXTENSION"
} | tee "$WORK_DIR/dynamic-dependencies.txt"

printf 'cuda-preflight: stage installed N-API package\n'
(
  cd "$REPO_ROOT/src/ts"
  npm exec -- tsc -p tsconfig.build.json
)
NPM_STAGING="$WORK_DIR/npm-artifacts"
NPM_MAIN="$NPM_STAGING/main"
NPM_PLATFORM_ROOT="$NPM_STAGING/platforms"
NPM_PLATFORM="$NPM_PLATFORM_ROOT/linux-x64-gnu"
mkdir -p "$NPM_MAIN" "$NPM_PLATFORM"
cp "$REPO_ROOT/src/ts/package.json" "$REPO_ROOT/src/ts/LICENSE" "$NPM_MAIN/"
cp -R "$REPO_ROOT/src/ts/dist" "$NPM_MAIN/dist"
cp "$REPO_ROOT/src/ts/npm/linux-x64-gnu/package.json" "$REPO_ROOT/src/ts/npm/linux-x64-gnu/LICENSE" "$NPM_PLATFORM/"
cp "$NAPI_BINARY" "$NPM_PLATFORM/fathomdb.linux-x64-gnu.node"
bash "$SCRIPT_DIR/npm-inject-optional-deps.sh" "$NPM_MAIN" "$NPM_PLATFORM_ROOT"
NPM_PLATFORM_TARBALL="$(cd "$NPM_PLATFORM" && npm pack --silent)"
NPM_MAIN_TARBALL="$(cd "$NPM_MAIN" && npm pack --silent)"

for smoke in driverless_python driverless_napi gpu_python gpu_napi; do
  initial_entry=""
  mkdir -p "$WORK_DIR/cache/$smoke" "$WORK_DIR/tmp/$smoke"
  initial_entry="$(find "$WORK_DIR/cache/$smoke" -mindepth 1 -print -quit)"
  if [ -n "$initial_entry" ]; then
    printf 'cuda-preflight: product cache was not empty before %s\n' "$smoke" >&2
    exit 1
  fi
done

MODEL_ENV=(
  -e HF_HOME=/fathomdb-hf
  -e XDG_CACHE_HOME=/fathomdb-product-cache
  -e HOME=/fathomdb-unavailable-home
  -e TMPDIR=/fathomdb-tmp
)
RERANKER_MOUNT=()
RERANKER_ENV=()
if [ "$RERANK_CUDA" = true ]; then
  RERANKER_MOUNT=(--mount "type=bind,src=$DEFAULT_RERANKER_CACHE_ROOT,dst=/fathomdb-reranker-cache-root,readonly")
  RERANKER_ENV=(
    -e FATHOMDB_RERANKER_CACHE=/fathomdb-reranker-cache-root
    -e FATHOMDB_CUDA_REHEARSAL_RERANK=true
  )
fi
printf 'cuda-preflight: prove the installed Python wheel defaults to CPU in a driverless container\n'
docker run --rm --network none \
  --mount "type=bind,src=$WHEEL,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  --mount "type=bind,src=$WORK_DIR/cache/driverless_python,dst=/fathomdb-product-cache" \
  --mount "type=bind,src=$WORK_DIR/tmp/driverless_python,dst=/fathomdb-tmp" \
  "${MODEL_ENV[@]}" \
  "${RERANKER_MOUNT[@]}" "${RERANKER_ENV[@]}" \
  "$CUDA_DRIVERLESS_PYTHON_IMAGE" \
  env -u FATHOMDB_EMBED_DEVICE -u FATHOMDB_RERANK_DEVICE -u CUDA_VISIBLE_DEVICES -u NVIDIA_VISIBLE_DEVICES -u HIP_VISIBLE_DEVICES -u ROCR_VISIBLE_DEVICES -u HUGGINGFACE_HUB_CACHE -u TRANSFORMERS_CACHE -u FATHOMDB_EMBEDDER_CACHE_DIR sh -ceu '
    test ! -e /dev/nvidiactl
    python -m pip install --no-deps --no-cache-dir /input/fathomdb.whl
    python - <<"PY"
import pathlib
import tempfile
from fathomdb import Engine
with tempfile.TemporaryDirectory(dir="/fathomdb-tmp") as directory:
    engine = Engine.open(str(pathlib.Path(directory) / "driverless.fdb"), use_default_embedder=True)
    assert engine.open_report().embedder_device_resolution.effective_device.kind == "cpu"
    assert len(engine.embed("driverless Python CPU fallback proof")) == 384
    engine.close()
if __import__("os").environ.get("FATHOMDB_CUDA_REHEARSAL_RERANK") == "true":
    from fathomdb import rerank
    result = rerank("reranker CPU proof", [{"id": 1, "body": "TinyBERT CPU inference", "score": 1.0}], 1)
    assert len(result) == 1 and result[0]["ce_score"] is not None
print("driverless installed Python CUDA-capable default-embedder CPU smoke: ok")
PY
  ' | tee "$WORK_DIR/driverless-python-cpu-smoke.txt"

printf 'cuda-preflight: prove the installed N-API package defaults to CPU in a driverless container\n'
docker run --rm --network none \
  --mount "type=bind,src=$NPM_MAIN/$NPM_MAIN_TARBALL,dst=/input/fathomdb.tgz,readonly" \
  --mount "type=bind,src=$NPM_PLATFORM/$NPM_PLATFORM_TARBALL,dst=/input/fathomdb-linux-x64-gnu.tgz,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  --mount "type=bind,src=$WORK_DIR/cache/driverless_napi,dst=/fathomdb-product-cache" \
  --mount "type=bind,src=$WORK_DIR/tmp/driverless_napi,dst=/fathomdb-tmp" \
  --mount "type=bind,src=$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64,dst=/opt/cuda/lib64,readonly" \
  "${MODEL_ENV[@]}" \
  "${RERANKER_MOUNT[@]}" "${RERANKER_ENV[@]}" \
  -e LD_LIBRARY_PATH=/opt/cuda/lib64 -e npm_config_cache=/fathomdb-tmp/npm-cache \
  "$CUDA_DRIVERLESS_NODE_IMAGE" \
  env -u FATHOMDB_EMBED_DEVICE -u FATHOMDB_RERANK_DEVICE -u CUDA_VISIBLE_DEVICES -u NVIDIA_VISIBLE_DEVICES -u HIP_VISIBLE_DEVICES -u ROCR_VISIBLE_DEVICES -u HUGGINGFACE_HUB_CACHE -u TRANSFORMERS_CACHE -u FATHOMDB_EMBEDDER_CACHE_DIR sh -ceu '
    test ! -e /dev/nvidiactl
    mkdir /fathomdb-tmp/consumer && cd /fathomdb-tmp/consumer
    printf "{\"private\":true,\"type\":\"module\",\"dependencies\":{\"fathomdb\":\"file:/input/fathomdb.tgz\",\"fathomdb-linux-x64-gnu\":\"file:/input/fathomdb-linux-x64-gnu.tgz\"}}\n" > package.json
    npm install --offline --ignore-scripts --no-audit --no-fund
    node --input-type=module - <<"JS"
import { Engine } from "fathomdb";
const engine = await Engine.open("/fathomdb-tmp/driverless-node.fdb", { useDefaultEmbedder: true });
if (engine.openReport().embedderDeviceResolution.effectiveDevice.kind !== "cpu") throw new Error("expected CPU fallback");
if ((await engine.embed("driverless N-API CPU fallback proof")).length !== 384) throw new Error("expected 384-vector");
if (process.env.FATHOMDB_CUDA_REHEARSAL_RERANK === "true") {
  await engine.write([{kind: "doc", body: "TinyBERT CPU inference", sourceId: "reranker-cpu-proof"}]);
  const result = await engine.search("reranker CPU proof", undefined, 1);
  if (result.results.length !== 1 || result.results[0].ceScore === null) throw new Error("expected installed N-API reranker inference");
}
await engine.close();
console.log("driverless installed N-API CUDA-capable default-embedder CPU smoke: ok");
JS
  ' | tee "$WORK_DIR/driverless-napi-cpu-smoke.txt"

cp "$SCRIPT_DIR/forced-python-open.py" "$WORK_DIR/forced-python-open.py"
cp "$SCRIPT_DIR/forced-napi-open.mjs" "$WORK_DIR/forced-napi-open.mjs"
if [ "$RERANK_CUDA" = true ]; then
  cp "$SCRIPT_DIR/forced-reranker-python.py" "$WORK_DIR/forced-reranker-python.py"
  cp "$SCRIPT_DIR/forced-reranker-napi.mjs" "$WORK_DIR/forced-reranker-napi.mjs"
fi
FORCED_PYTHON_SITE="$WORK_DIR/forced-python-site"
mkdir "$FORCED_PYTHON_SITE"
docker run --rm --network none \
  --mount "type=bind,src=$WHEEL,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$FORCED_PYTHON_SITE,dst=/fathomdb-site" \
  -e HOME=/tmp -e TMPDIR=/tmp \
  "$CUDA_MANYLINUX_IMAGE" sh -ceu '
    /opt/python/cp311-cp311/bin/python -m pip install \
      --no-deps --no-cache-dir --target /fathomdb-site /input/fathomdb.whl
    chmod -R a+rwX /fathomdb-site
  ' \
  >"$WORK_DIR/forced-python-install.txt" 2>&1

FORCED_NAPI_INSTALL="$WORK_DIR/forced-napi-install"
mkdir "$FORCED_NAPI_INSTALL"
cat > "$FORCED_NAPI_INSTALL/package.json" <<EOF
{"private":true,"type":"module","dependencies":{"fathomdb":"file:$NPM_MAIN/$NPM_MAIN_TARBALL","fathomdb-linux-x64-gnu":"file:$NPM_PLATFORM/$NPM_PLATFORM_TARBALL"}}
EOF
(
  cd "$FORCED_NAPI_INSTALL"
  npm install --offline --ignore-scripts --no-audit --no-fund
)

run_forced_python() {
  local stdout="$WORK_DIR/forced-cuda-unavailable-python-stdout.txt"
  local stderr="$WORK_DIR/forced-cuda-unavailable-python-stderr.txt"
  set +e
  docker run --rm --network none \
    --mount "type=bind,src=$FORCED_PYTHON_SITE,dst=/fathomdb-site,readonly" \
    --mount "type=bind,src=$WORK_DIR/forced-python-open.py,dst=/fathomdb-harness/forced-python-open.py,readonly" \
    -e HOME=/fathomdb-unavailable-home -e TMPDIR=/tmp -e PYTHONPATH=/fathomdb-site -e FATHOMDB_EMBED_DEVICE=cuda:0 \
    "$CUDA_MANYLINUX_IMAGE" sh -ceu '
      exec /opt/python/cp311-cp311/bin/python /fathomdb-harness/forced-python-open.py
    ' >"$stdout" 2>"$stderr"
  local exit_code="$?"
  set -e
  [ "$exit_code" -eq 1 ] || {
    printf 'cuda-preflight: forced Python cuda:0 did not fail with exit 1\n' >&2
    exit 1
  }
}

run_forced_napi() {
  local stdout="$WORK_DIR/forced-cuda-unavailable-napi-stdout.txt"
  local stderr="$WORK_DIR/forced-cuda-unavailable-napi-stderr.txt"
  set +e
  docker run --rm --network none \
    --mount "type=bind,src=$WORK_DIR/forced-napi-open.mjs,dst=/fathomdb-harness/forced-napi-open.mjs,readonly" \
    --mount "type=bind,src=$FORCED_NAPI_INSTALL/node_modules,dst=/fathomdb-harness/node_modules,readonly" \
    --mount "type=bind,src=$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64,dst=/opt/cuda/lib64,readonly" \
    -e HOME=/fathomdb-unavailable-home -e TMPDIR=/tmp -e FATHOMDB_EMBED_DEVICE=cuda:0 -e LD_LIBRARY_PATH=/opt/cuda/lib64 \
    "$CUDA_DRIVERLESS_NODE_IMAGE" sh -ceu '
      exec node /fathomdb-harness/forced-napi-open.mjs
    ' >"$stdout" 2>"$stderr"
  local exit_code="$?"
  set -e
  [ "$exit_code" -eq 1 ] || {
    printf 'cuda-preflight: forced N-API cuda:0 did not fail with exit 1\n' >&2
    exit 1
  }
}

run_forced_python
run_forced_napi

run_forced_reranker_python() {
  local stdout="$WORK_DIR/forced-reranker-python-stdout.txt"
  local stderr="$WORK_DIR/forced-reranker-python-stderr.txt"
  set +e
  docker run --rm --network none \
    --mount "type=bind,src=$FORCED_PYTHON_SITE,dst=/fathomdb-site,readonly" \
    --mount "type=bind,src=$WORK_DIR/forced-reranker-python.py,dst=/fathomdb-harness/forced-reranker-python.py,readonly" \
    -e HOME=/fathomdb-unavailable-home -e TMPDIR=/tmp -e PYTHONPATH=/fathomdb-site -e FATHOMDB_RERANK_DEVICE=cuda:0 \
    "$CUDA_MANYLINUX_IMAGE" sh -ceu '
      exec /opt/python/cp311-cp311/bin/python /fathomdb-harness/forced-reranker-python.py
    ' >"$stdout" 2>"$stderr"
  local exit_code="$?"
  set -e
  [ "$exit_code" -eq 1 ] || {
    printf 'cuda-preflight: forced Python reranker cuda:0 did not fail with exit 1\n' >&2
    exit 1
  }
}

run_forced_reranker_napi() {
  local stdout="$WORK_DIR/forced-reranker-napi-stdout.txt"
  local stderr="$WORK_DIR/forced-reranker-napi-stderr.txt"
  set +e
  docker run --rm --network none \
    --mount "type=bind,src=$WORK_DIR/forced-reranker-napi.mjs,dst=/fathomdb-harness/forced-reranker-napi.mjs,readonly" \
    --mount "type=bind,src=$FORCED_NAPI_INSTALL/node_modules,dst=/fathomdb-harness/node_modules,readonly" \
    --mount "type=bind,src=$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64,dst=/opt/cuda/lib64,readonly" \
    -e HOME=/fathomdb-unavailable-home -e TMPDIR=/tmp -e FATHOMDB_RERANK_DEVICE=cuda:0 -e LD_LIBRARY_PATH=/opt/cuda/lib64 \
    "$CUDA_DRIVERLESS_NODE_IMAGE" sh -ceu '
      exec node /fathomdb-harness/forced-reranker-napi.mjs
    ' >"$stdout" 2>"$stderr"
  local exit_code="$?"
  set -e
  [ "$exit_code" -eq 1 ] || {
    printf 'cuda-preflight: forced N-API reranker cuda:0 did not fail with exit 1\n' >&2
    exit 1
  }
}

if [ "$RERANK_CUDA" = true ]; then
  run_forced_reranker_python
  run_forced_reranker_napi
fi

cat > "$WORK_DIR/gpu-python-smoke.py" <<'PY'
from dataclasses import asdict
import json
import pathlib
import time
from fathomdb import Engine
engine = Engine.open("/fathomdb-tmp/cuda-python.fdb", use_default_embedder=True)
report = engine.open_report()
resolution = report.embedder_device_resolution
allocation_witness = report.embedder_gpu_allocation_witness
if allocation_witness is None:
    raise RuntimeError("installed Python CUDA artifact did not retain an allocation witness")
pathlib.Path("/evidence/gpu-python-open-report.json").write_text(json.dumps({
    "consumer": "python", "requested_policy": resolution.requested_policy,
    "status": "selected_cuda", "effective_device": "cuda:0",
    "visible_devices": [{"visible_ordinal": d.visible_ordinal, "uuid": d.uuid, "name": d.name, "compute_capability": d.compute_capability} for d in resolution.visible_cuda_devices],
    "selected_uuid": resolution.selected_cuda_uuid,
    "allocation_witness": asdict(allocation_witness),
}, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
assert len(engine.embed("installed Python CUDA artifact GPU proof")) == 384
print("installed Python CUDA artifact GPU smoke: ok", flush=True)
time.sleep(20)
engine.close()
PY

cat > "$WORK_DIR/gpu-napi-smoke.mjs" <<'JS'
import fs from "node:fs";
import { Engine } from "fathomdb";
const engine = await Engine.open("/fathomdb-tmp/cuda-napi.fdb", { useDefaultEmbedder: true });
const report = engine.openReport();
const resolution = report.embedderDeviceResolution;
const allocationWitness = report.embedderGpuAllocationWitness;
if (allocationWitness === null) throw new Error("installed N-API CUDA artifact did not retain an allocation witness");
fs.writeFileSync("/evidence/gpu-napi-open-report.json", JSON.stringify({
  consumer: "napi", requested_policy: resolution.requestedPolicy,
  status: "selected_cuda", effective_device: "cuda:0",
  visible_devices: resolution.visibleCudaDevices.map((d) => ({ visible_ordinal: d.visibleOrdinal, uuid: d.uuid, name: d.name, compute_capability: d.computeCapability })),
  selected_uuid: resolution.selectedCudaUuid,
  allocation_witness: {
    schema: allocationWitness.schema,
    sole_gpu_consumer_precondition: allocationWitness.soleGpuConsumerPrecondition,
    device_ordinal_requested: allocationWitness.deviceOrdinalRequested,
    device_ordinal_actual: allocationWitness.deviceOrdinalActual,
    device_uuid: allocationWitness.deviceUuid,
    device_name: allocationWitness.deviceName,
    compute_capability: allocationWitness.computeCapability,
    free_before_bytes: allocationWitness.freeBeforeBytes,
    free_after_bytes: allocationWitness.freeAfterBytes,
    total_bytes: allocationWitness.totalBytes,
    delta_bytes: allocationWitness.deltaBytes,
    delta_floor_bytes: allocationWitness.deltaFloorBytes,
    control_allocation_request_bytes: allocationWitness.controlAllocationRequestBytes,
    control_block_count: allocationWitness.controlBlockCount,
    control_free_before_bytes: allocationWitness.controlFreeBeforeBytes,
    control_free_after_bytes: allocationWitness.controlFreeAfterBytes,
    control_delta_bytes: allocationWitness.controlDeltaBytes,
    embedded_vector_dim: allocationWitness.embeddedVectorDim,
  },
}) + "\n");
if ((await engine.embed("installed N-API CUDA artifact GPU proof")).length !== 384) throw new Error("expected 384-vector");
console.log("installed N-API CUDA artifact GPU smoke: ok");
await new Promise((resolve) => setTimeout(resolve, 20_000));
await engine.close();
JS

seal_gpu_observation() {
  local container="$1" consumer="$2" report="$3" smoke="$4"
  local host_pid compute_line host_uuid process_name exit_code
  for _ in $(seq 1 30); do
    docker inspect --format '{{.State.Running}}' "$container" | grep -Fx true >/dev/null || {
      docker logs "$container" >&2 || true
      printf 'cuda-preflight: %s GPU smoke exited before observation\n' "$consumer" >&2
      exit 1
    }
    host_pid="$(docker inspect --format '{{.State.Pid}}' "$container")"
    compute_line="$(nvidia-smi --id=0 --query-compute-apps=pid,process_name --format=csv,noheader | awk -F ', ' -v pid="$host_pid" '$1 == pid {print; exit}')"
    if [ -n "$compute_line" ] && [ -s "$report" ]; then
      process_name="${compute_line#*, }"
      host_uuid="$(nvidia-smi --id=0 --query-gpu=uuid --format=csv,noheader)"
      python3 - "$report" "$WORK_DIR/gpu-$consumer-cuda-witness.json" "$host_uuid" "$host_pid" "$process_name" <<'PY'
import json
import sys
from pathlib import Path
source, target, host_uuid, host_pid, process_name = sys.argv[1:]
value = json.loads(Path(source).read_text())
value.update({
    "schema_version": "fathomdb.cuda-device-observation/v1",
    "nvidia_smi_uuid": host_uuid,
    "process_id": int(host_pid),
    "nvidia_smi_compute_process_id": int(host_pid),
    "process_name": process_name,
})
Path(target).write_text(json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
      exit_code="$(docker wait "$container")"
      docker logs "$container" >"$smoke" 2>&1
      docker rm "$container" >/dev/null
      [ "$exit_code" = 0 ] || {
        printf 'cuda-preflight: %s GPU smoke failed after observation\n' "$consumer" >&2
        exit 1
      }
      return
    fi
    sleep 1
  done
  docker logs "$container" >&2 || true
  docker rm "$container" >/dev/null 2>&1 || true
  printf 'cuda-preflight: %s GPU smoke lacked UUID/PID evidence\n' "$consumer" >&2
  exit 1
}

printf 'cuda-preflight: prove the installed Python wheel uses cuda:0\n'
PYTHON_GPU_CONTAINER="$(docker run -d --gpus '"'"'device=0'"'"' --network none \
  --mount "type=bind,src=$WHEEL,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$WORK_DIR/gpu-python-smoke.py,dst=/input/gpu-python-smoke.py,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  --mount "type=bind,src=$WORK_DIR/cache/gpu_python,dst=/fathomdb-product-cache" \
  --mount "type=bind,src=$WORK_DIR/tmp/gpu_python,dst=/fathomdb-tmp" \
  --mount "type=bind,src=$WORK_DIR,dst=/evidence" \
  "${MODEL_ENV[@]}" -e FATHOMDB_EMBED_DEVICE=cuda:0 -e FATHOMDB_GPU_ALLOCATION_WITNESS=1 \
  "$CUDA_MANYLINUX_IMAGE" sh -ceu '
    '"$CUDA_MANYLINUX_PYTHON"' -m pip install --no-deps --no-cache-dir /input/fathomdb.whl
    exec '"$CUDA_MANYLINUX_PYTHON"' /input/gpu-python-smoke.py
  ')"
seal_gpu_observation "$PYTHON_GPU_CONTAINER" python "$WORK_DIR/gpu-python-open-report.json" "$WORK_DIR/gpu-python-cuda-smoke.txt"

printf 'cuda-preflight: prove the installed N-API package uses cuda:0\n'
NAPI_GPU_CONTAINER="$(docker run -d --gpus '"'"'device=0'"'"' --network none \
  --mount "type=bind,src=$NPM_MAIN/$NPM_MAIN_TARBALL,dst=/input/fathomdb.tgz,readonly" \
  --mount "type=bind,src=$NPM_PLATFORM/$NPM_PLATFORM_TARBALL,dst=/input/fathomdb-linux-x64-gnu.tgz,readonly" \
  --mount "type=bind,src=$WORK_DIR/gpu-napi-smoke.mjs,dst=/input/gpu-napi-smoke.mjs,readonly" \
  --mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly" \
  --mount "type=bind,src=$WORK_DIR/cache/gpu_napi,dst=/fathomdb-product-cache" \
  --mount "type=bind,src=$WORK_DIR/tmp/gpu_napi,dst=/fathomdb-tmp" \
  --mount "type=bind,src=$WORK_DIR,dst=/evidence" \
  --mount "type=bind,src=$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64,dst=/opt/cuda/lib64,readonly" \
  "${MODEL_ENV[@]}" -e FATHOMDB_EMBED_DEVICE=cuda:0 -e FATHOMDB_GPU_ALLOCATION_WITNESS=1 -e LD_LIBRARY_PATH=/opt/cuda/lib64 -e npm_config_cache=/fathomdb-tmp/npm-cache \
  "$CUDA_DRIVERLESS_NODE_IMAGE" sh -ceu '
    mkdir /fathomdb-tmp/consumer && cd /fathomdb-tmp/consumer
    printf "{\"private\":true,\"type\":\"module\",\"dependencies\":{\"fathomdb\":\"file:/input/fathomdb.tgz\",\"fathomdb-linux-x64-gnu\":\"file:/input/fathomdb-linux-x64-gnu.tgz\"}}\n" > package.json
    npm install --offline --ignore-scripts --no-audit --no-fund
    cp /input/gpu-napi-smoke.mjs ./gpu-napi-smoke.mjs
    exec node ./gpu-napi-smoke.mjs
  ')"
seal_gpu_observation "$NAPI_GPU_CONTAINER" napi "$WORK_DIR/gpu-napi-open-report.json" "$WORK_DIR/gpu-napi-cuda-smoke.txt"

CANDIDATE_SHA="$(git -C "$REPO_ROOT" rev-parse HEAD)"
export WORK_DIR CANDIDATE_SHA CUDA_DEFAULT_EMBEDDER_HF_REPO CUDA_DEFAULT_EMBEDDER_HF_REVISION RERANK_CUDA
export CUDA_DEFAULT_EMBEDDER_CONFIG_SHA256 CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256 CUDA_DEFAULT_EMBEDDER_MODEL_SHA256
export CUDA_RERANKER_CONFIG_SHA256 CUDA_RERANKER_TOKENIZER_SHA256 CUDA_RERANKER_MODEL_SHA256
python3 - <<'PY'
import hashlib
import json
import os
from pathlib import Path

root = Path(os.environ["WORK_DIR"])
canonical = lambda value: json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n"
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
repository = os.environ["CUDA_DEFAULT_EMBEDDER_HF_REPO"]
revision = os.environ["CUDA_DEFAULT_EMBEDDER_HF_REVISION"]
files = {
    "config.json": os.environ["CUDA_DEFAULT_EMBEDDER_CONFIG_SHA256"],
    "tokenizer.json": os.environ["CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256"],
    "model.safetensors": os.environ["CUDA_DEFAULT_EMBEDDER_MODEL_SHA256"],
}
manifest = {
    "schema_version": "fathomdb.cuda-model-cache/v1",
    "repository": repository,
    "revision": revision,
    "snapshot_relpath": f"hub/models--{repository.replace('/', '--')}/snapshots/{revision}",
    "files": files,
}
(root / "model-cache-manifest.json").write_text(canonical(manifest))
model_digest = digest(root / "model-cache-manifest.json")
rerank_cuda = os.environ["RERANK_CUDA"] == "true"
build = {
    "schema_version": "fathomdb.cuda-preflight-build-input/v3" if rerank_cuda else "fathomdb.cuda-preflight-build-input/v2",
    "candidate_sha": os.environ["CANDIDATE_SHA"],
    "target": "x86_64-unknown-linux-gnu",
    "python_features": ["embed-cuda", "rerank-cuda", "pyo3/extension-module"] if rerank_cuda else ["embed-cuda", "pyo3/extension-module"],
    "napi_features": ["default-embedder", "embed-cuda", "rerank-cuda"] if rerank_cuda else ["default-embedder", "embed-cuda"],
    "rerank_cuda": rerank_cuda,
    "model_cache_manifest_sha256": model_digest,
}
(root / "build-input.json").write_text(canonical(build))

for consumer in ("python", "napi"):
    stdout = root / f"forced-cuda-unavailable-{consumer}-stdout.txt"
    stderr = root / f"forced-cuda-unavailable-{consumer}-stderr.txt"
    record = {
        "schema_version": "fathomdb.cuda-forced-device-failure/v1",
        "consumer": consumer,
        "requested_policy": "cuda:0",
        "cuda_compiled": True,
        "visible_devices": [],
        "status": "cuda_unavailable",
        "effective_device": None,
        "reason": "no_visible_cuda_device",
        "provenance": "installed_candidate",
        "command": f"installed_{consumer}_engine_open_without_default_embedder",
        "exit_code": 1,
        "stdout_filename": stdout.name,
        "stdout_sha256": digest(stdout),
        "stderr_filename": stderr.name,
        "stderr_sha256": digest(stderr),
    }
    (root / f"forced-cuda-unavailable-{consumer}.json").write_text(canonical(record))

prefix = hashlib.sha256(f"{repository}@{revision}".encode()).hexdigest()[:12]
expected_files = {f"fathomdb/embedders/{prefix}/{name}": value for name, value in files.items()}
smokes = {}
for name in ("driverless_python", "driverless_napi", "gpu_python", "gpu_napi"):
    cache_root = root / "cache" / name
    all_files = {
        path.relative_to(cache_root).as_posix(): digest(path)
        for path in sorted(cache_root.rglob("*")) if path.is_file() and not path.is_symlink()
    }
    lock_name = f"fathomdb/embedders/{prefix}/.lock"
    if set(all_files) - ({lock_name} | set(expected_files)):
        raise SystemExit(f"cuda-preflight: {name} product cache has unexpected files: {all_files}")
    actual = {path: all_files[path] for path in expected_files if path in all_files}
    if actual != expected_files:
        raise SystemExit(f"cuda-preflight: {name} product-cache materialization differs: {actual}")
    smokes[name] = {
        "hf_home": "/fathomdb-hf",
        "hf_seed_read_only": True,
        "xdg_cache_home": "/fathomdb-product-cache",
        "network": "none",
        "product_cache_initial_entries": 0,
        "product_cache_files": actual,
    }
(root / "smoke-cache-topology.json").write_text(canonical({
    "schema_version": "fathomdb.cuda-smoke-cache-topology/v1", "smokes": smokes,
}))

if rerank_cuda:
    reranker_files = {
        "config.json": os.environ["CUDA_RERANKER_CONFIG_SHA256"],
        "tokenizer.json": os.environ["CUDA_RERANKER_TOKENIZER_SHA256"],
        "model.safetensors": os.environ["CUDA_RERANKER_MODEL_SHA256"],
    }
    reranker_manifest = {
        "schema_version": "fathomdb.reranker-cache/v1",
        "repository": "cross-encoder/ms-marco-TinyBERT-L2-v2",
        "revision": "81d1926f67cb8eee2c2be17ca9f793c7c3bd20cc",
        "snapshot_relpath": "fathomdb/reranker/0290849b0459",
        "files": reranker_files,
    }
    (root / "reranker-cache-manifest.json").write_text(canonical(reranker_manifest))
    reranker_manifest_digest = digest(root / "reranker-cache-manifest.json")
    for consumer in ("python", "napi"):
        record = {
            "schema_version": "fathomdb.cuda-reranker-cpu-smoke/v1",
            "consumer": consumer,
            "requested_policy": "auto",
            "effective_device": "cpu",
            "reason": "no_visible_cuda_device",
            "network": "none",
            "source_imported": False,
            "rerank_performed": True,
            "reranker_cache_manifest_sha256": reranker_manifest_digest,
            "reranker_cache_read_only": True,
            "reranker_device_environment": "unset",
        }
        (root / f"reranker-{consumer}-cpu-smoke.json").write_text(canonical(record))
    for consumer in ("python", "napi"):
        stdout = root / f"forced-reranker-{consumer}-stdout.txt"
        stderr = root / f"forced-reranker-{consumer}-stderr.txt"
        record = {
            "schema_version": "fathomdb.cuda-forced-reranker-failure/v1",
            "consumer": consumer,
            "requested_policy": "cuda:0",
            "cuda_compiled": True,
            "visible_devices": [],
            "status": "cuda_unavailable",
            "effective_device": None,
            "reason": "no_visible_cuda_device",
            "provenance": "installed_candidate",
            "command": f"installed_{consumer}_engine_open",
            "exit_code": 1,
            "stdout_filename": stdout.name,
            "stdout_sha256": digest(stdout),
            "stderr_filename": stderr.name,
            "stderr_sha256": digest(stderr),
        }
        (root / f"forced-reranker-{consumer}.json").write_text(canonical(record))
PY

EVIDENCE_NAMES=(
  environment.txt manylinux-build.txt dynamic-dependencies.txt python-auditwheel.txt
  driverless-python-cpu-smoke.txt driverless-napi-cpu-smoke.txt
  gpu-python-cuda-witness.json gpu-napi-cuda-witness.json
  gpu-python-cuda-smoke.txt gpu-napi-cuda-smoke.txt
  build-input.json model-cache-manifest.json smoke-cache-topology.json
  forced-python-open.py forced-napi-open.mjs
  forced-cuda-unavailable-python.json forced-cuda-unavailable-napi.json
  forced-cuda-unavailable-python-stdout.txt forced-cuda-unavailable-python-stderr.txt
  forced-cuda-unavailable-napi-stdout.txt forced-cuda-unavailable-napi-stderr.txt
)
if [ "$RERANK_CUDA" = true ]; then
  EVIDENCE_NAMES+=(
    reranker-cache-manifest.json reranker-python-cpu-smoke.json reranker-napi-cpu-smoke.json
    forced-reranker-python.json forced-reranker-napi.json
    forced-reranker-python-stdout.txt forced-reranker-python-stderr.txt
    forced-reranker-napi-stdout.txt forced-reranker-napi-stderr.txt
    forced-reranker-python.py forced-reranker-napi.mjs
  )
fi
mkdir "$OUTPUT_DIR"
for name in "${EVIDENCE_NAMES[@]}"; do
  [ -f "$WORK_DIR/$name" ] && [ -s "$WORK_DIR/$name" ] || {
    printf 'cuda-preflight: required evidence absent or empty: %s\n' "$name" >&2
    exit 1
  }
  cp "$WORK_DIR/$name" "$OUTPUT_DIR/$name"
done
export OUTPUT_DIR
python3 - <<'PY'
import hashlib
import json
import os
from pathlib import Path
root = Path(os.environ["OUTPUT_DIR"])
evidence = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(root.iterdir())}
payload = {
    "schema_version": "fathomdb.cuda-preflight-witness/v3" if os.environ["RERANK_CUDA"] == "true" else "fathomdb.cuda-preflight-witness/v2",
    "candidate_sha": os.environ["CANDIDATE_SHA"],
    "outcome": "passed",
    "build_input_sha256": evidence["build-input.json"],
    "model_cache_manifest_sha256": evidence["model-cache-manifest.json"],
    "evidence_sha256": evidence,
}
(root / "cuda-preflight-witness.json").write_text(
    json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n"
)
PY
python3 "$SCRIPT_DIR/verify-cuda-preflight-witness.py" \
  --witness-dir "$OUTPUT_DIR" \
  --candidate-sha "$CANDIDATE_SHA"
if [ "$RERANK_CUDA" = true ]; then
  PACKAGE_DIR="${OUTPUT_DIR}.packages"
  mkdir "$PACKAGE_DIR"
  cp "$WHEEL" "$PACKAGE_DIR/"
  cp "$NPM_MAIN/$NPM_MAIN_TARBALL" "$PACKAGE_DIR/"
  cp "$NPM_PLATFORM/$NPM_PLATFORM_TARBALL" "$PACKAGE_DIR/"
fi
printf 'cuda-preflight: pass; witness at %s\n' "$OUTPUT_DIR"
