#!/usr/bin/env bash
# Run installed-package-only CPU and GPU smokes for a CUDA package rehearsal.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"

usage() {
  printf 'usage: %s --python-wheel FILE --npm-main FILE --napi-platform FILE --hf-home DIR --smoke-dir DIR\n' "$0" >&2
}

python_wheel='' npm_main='' napi_platform='' hf_home='' smoke_dir=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --python-wheel) python_wheel="$2" ;;
    --npm-main) npm_main="$2" ;;
    --napi-platform) napi_platform="$2" ;;
    --hf-home) hf_home="$2" ;;
    --smoke-dir) smoke_dir="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done
for value in python_wheel npm_main napi_platform hf_home smoke_dir; do
  [ -n "${!value}" ] || { usage; exit 2; }
done
for path in "$python_wheel" "$npm_main" "$napi_platform"; do
  [ -f "$path" ] && [ ! -L "$path" ] || { printf 'cuda-package-smoke: package input is absent or symlinked: %s\n' "$path" >&2; exit 1; }
done
[ ! -e "$smoke_dir" ] || { printf 'cuda-package-smoke: smoke directory must be new: %s\n' "$smoke_dir" >&2; exit 1; }
[ -d "$hf_home" ] && [ ! -L "$hf_home" ] || { printf 'cuda-package-smoke: pinned embedder cache must be a non-symlink directory\n' >&2; exit 1; }
python_wheel_abs="$(realpath -- "$python_wheel")"
npm_main_abs="$(realpath -- "$npm_main")"
napi_platform_abs="$(realpath -- "$napi_platform")"
hf_home_abs="$(realpath -- "$hf_home")"
for command in docker nvidia-smi; do
  command -v "$command" >/dev/null || { printf 'cuda-package-smoke: missing %s\n' "$command" >&2; exit 1; }
done
docker info >/dev/null
mkdir -p "$smoke_dir"

write_cpu() {
  local consumer="$1"
  python3 - "$smoke_dir/cpu-$consumer.json" "$consumer" <<'PY'
import json
from pathlib import Path
import sys

Path(sys.argv[1]).write_text(json.dumps({
    "schema_version": "fathomdb.cuda-package-cpu-smoke/v1",
    "consumer": sys.argv[2], "network": "none", "environment": "env -i",
    "gpu_nodes_visible": False, "source_imported": False, "outcome": "passed",
}, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
}

write_gpu() {
  local consumer="$1" pid="$2" uuid="$3" index="$4" name="$5" driver="$6"
  python3 - "$smoke_dir/gpu-$consumer.json" "$consumer" "$pid" "$uuid" "$index" "$name" "$driver" <<'PY'
import json
from pathlib import Path
import sys

Path(sys.argv[1]).write_text(json.dumps({
    "schema_version": "fathomdb.cuda-package-gpu-smoke/v1",
    "consumer": sys.argv[2], "network": "none", "source_imported": False, "outcome": "passed",
    "smoke_pid": int(sys.argv[3]), "nvidia_smi_pid": int(sys.argv[3]), "gpu_uuid": sys.argv[4],
    "nvidia_smi_uuid": sys.argv[4], "host_index": int(sys.argv[5]), "requested_ordinal": 0,
    "device_name": sys.argv[6], "driver_version": sys.argv[7],
}, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
}

# These commands mount only retained artifact bytes. The host checkout is
# never mounted; env -i and --network none prevent ambient package/download or
# device-selector inputs from satisfying a CPU-default smoke.
docker run --rm --network none \
  --mount "type=bind,src=$python_wheel_abs,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$hf_home_abs,dst=/fathomdb-hf,readonly" \
  "$CUDA_DRIVERLESS_PYTHON_IMAGE" \
  env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp HF_HOME=/fathomdb-hf sh -ceu '
    test ! -e /dev/nvidiactl
    exec python -c '"'"'
import tempfile
from pathlib import Path
import subprocess
subprocess.run(["python", "-m", "pip", "install", "--no-deps", "/input/fathomdb.whl"], check=True)
from fathomdb import Engine
with tempfile.TemporaryDirectory() as d:
    engine=Engine.open(str(Path(d) / "cpu.fdb"), use_default_embedder=True)
    engine.embed("CUDA package rehearsal installed Python CPU smoke")
    engine.write([{"kind":"doc","body":"{}","source_id":"cuda-package-cpu"}])
    engine.search("smoke"); engine.close()
'"'"'
  '
write_cpu python

docker run --rm --network none \
  --mount "type=bind,src=$npm_main_abs,dst=/input/fathomdb.tgz,readonly" \
  --mount "type=bind,src=$napi_platform_abs,dst=/input/fathomdb-linux-x64-gnu.tgz,readonly" \
  --mount "type=bind,src=$hf_home_abs,dst=/fathomdb-hf,readonly" \
  "$CUDA_DRIVERLESS_NODE_IMAGE" \
  env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp HF_HOME=/fathomdb-hf sh -ceu '
    test ! -e /dev/nvidiactl
    mkdir /consumer && cd /consumer
    printf "%s\n" "{\"private\":true,\"type\":\"module\",\"dependencies\":{\"fathomdb\":\"file:/input/fathomdb.tgz\",\"fathomdb-linux-x64-gnu\":\"file:/input/fathomdb-linux-x64-gnu.tgz\"}}" > package.json
    npm install --offline --ignore-scripts --no-audit --no-fund
    node --input-type=module -e "import { Engine } from \"fathomdb\"; const e=await Engine.open(\"/tmp/cpu.fdb\",{useDefaultEmbedder:true}); await e.embed(\"CUDA package rehearsal installed N-API CPU smoke\"); await e.write([{kind:\"doc\",body:\"{}\",sourceId:\"cuda-package-cpu\"}]); await e.search(\"smoke\"); await e.close();"
  '
write_cpu napi

gpu_index=0
gpu_csv="$(nvidia-smi --id="$gpu_index" --query-gpu=uuid,name,driver_version --format=csv,noheader)"
IFS=',' read -r gpu_uuid gpu_name gpu_driver <<<"$gpu_csv"
gpu_uuid="${gpu_uuid# }"; gpu_name="${gpu_name# }"; gpu_driver="${gpu_driver# }"
[ -n "$gpu_uuid" ] && [ -n "$gpu_name" ] && [ -n "$gpu_driver" ] || { printf 'cuda-package-smoke: GPU %s has incomplete identity\n' "$gpu_index" >&2; exit 1; }

wait_for_gpu() {
  local container="$1" consumer="$2" host_pid='' observed=''
  for _ in $(seq 1 30); do
    docker inspect --format '{{.State.Running}}' "$container" | grep -Fx true >/dev/null || break
    host_pid="$(docker inspect --format '{{.State.Pid}}' "$container")"
    observed="$(nvidia-smi --id="$gpu_index" --query-compute-apps=pid --format=csv,noheader || true)"
    if printf '%s\n' "$observed" | grep -Fx "$host_pid" >/dev/null; then
      docker wait "$container" >/dev/null
      docker rm "$container" >/dev/null
      write_gpu "$consumer" "$host_pid" "$gpu_uuid" "$gpu_index" "$gpu_name" "$gpu_driver"
      return
    fi
    sleep 1
  done
  docker logs "$container" >&2 || true
  docker rm -f "$container" >/dev/null 2>&1 || true
  printf 'cuda-package-smoke: %s did not produce a matching GPU PID on selected device\n' "$consumer" >&2
  exit 1
}

python_gpu="$(docker run -d --gpus '"'"'device=0'"'"' --network none \
  --mount "type=bind,src=$python_wheel_abs,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$hf_home_abs,dst=/fathomdb-hf,readonly" \
  "$CUDA_MANYLINUX_IMAGE" sh -ceu '
    env -i PATH=/opt/python/cp311-cp311/bin:/usr/local/bin:/usr/bin:/bin HOME=/tmp HF_HOME=/fathomdb-hf FATHOMDB_EMBED_DEVICE=cuda:0 /opt/python/cp311-cp311/bin/python -m pip install --no-deps /input/fathomdb.whl
    exec env -i PATH=/opt/python/cp311-cp311/bin:/usr/local/bin:/usr/bin:/bin HOME=/tmp HF_HOME=/fathomdb-hf FATHOMDB_EMBED_DEVICE=cuda:0 /opt/python/cp311-cp311/bin/python -c "from fathomdb import Engine; import tempfile; from pathlib import Path; d=tempfile.TemporaryDirectory(); e=Engine.open(str(Path(d.name)/\"gpu.fdb\"),use_default_embedder=True); e.embed(\"CUDA package rehearsal installed Python GPU smoke\"); e.close(); import time; time.sleep(20)"
  ')"
wait_for_gpu "$python_gpu" python

napi_gpu="$(docker run -d --gpus '"'"'device=0'"'"' --network none \
  --mount "type=bind,src=$npm_main_abs,dst=/input/fathomdb.tgz,readonly" \
  --mount "type=bind,src=$napi_platform_abs,dst=/input/fathomdb-linux-x64-gnu.tgz,readonly" \
  --mount "type=bind,src=$hf_home_abs,dst=/fathomdb-hf,readonly" \
  "$CUDA_DRIVERLESS_NODE_IMAGE" sh -ceu '
    mkdir /consumer && cd /consumer
    printf "%s\n" "{\"private\":true,\"type\":\"module\",\"dependencies\":{\"fathomdb\":\"file:/input/fathomdb.tgz\",\"fathomdb-linux-x64-gnu\":\"file:/input/fathomdb-linux-x64-gnu.tgz\"}}" > package.json
    env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp HF_HOME=/fathomdb-hf npm install --offline --ignore-scripts --no-audit --no-fund
    exec env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp HF_HOME=/fathomdb-hf FATHOMDB_EMBED_DEVICE=cuda:0 node --input-type=module -e "import { Engine } from \"fathomdb\"; const e=await Engine.open(\"/tmp/gpu.fdb\",{useDefaultEmbedder:true}); await e.embed(\"CUDA package rehearsal installed N-API GPU smoke\"); await e.close(); await new Promise(resolve=>setTimeout(resolve,20000));"
  ')"
wait_for_gpu "$napi_gpu" napi
