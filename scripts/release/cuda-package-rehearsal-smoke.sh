#!/usr/bin/env bash
# Run installed-package-only CPU and GPU smokes for a CUDA package rehearsal.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=cuda-artifact-contract.sh
. "$SCRIPT_DIR/cuda-artifact-contract.sh"

usage() {
  printf 'usage: %s --python-wheel FILE --npm-main FILE --napi-platform FILE --cli-archive FILE --model-cache-manifest FILE --hf-home DIR --smoke-dir DIR [--reranker-cache-manifest FILE]\n' "$0" >&2
}

python_wheel='' npm_main='' napi_platform='' cli_archive='' model_cache_manifest='' hf_home='' smoke_dir='' reranker_cache_manifest=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --python-wheel) python_wheel="$2" ;;
    --npm-main) npm_main="$2" ;;
    --napi-platform) napi_platform="$2" ;;
    --cli-archive) cli_archive="$2" ;;
    --model-cache-manifest) model_cache_manifest="$2" ;;
    --hf-home) hf_home="$2" ;;
    --smoke-dir) smoke_dir="$2" ;;
    --reranker-cache-manifest) reranker_cache_manifest="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done
for value in python_wheel npm_main napi_platform cli_archive model_cache_manifest hf_home smoke_dir; do
  [ -n "${!value}" ] || { usage; exit 2; }
done
for path in "$python_wheel" "$npm_main" "$napi_platform" "$cli_archive" "$model_cache_manifest"; do
  [ -f "$path" ] && [ ! -L "$path" ] || { printf 'cuda-package-smoke: package input is absent or symlinked: %s\n' "$path" >&2; exit 1; }
done
if [ -n "$reranker_cache_manifest" ]; then
  [ -f "$reranker_cache_manifest" ] && [ ! -L "$reranker_cache_manifest" ] || { printf 'cuda-package-smoke: reranker cache manifest is absent or symlinked\n' >&2; exit 1; }
fi
[ ! -e "$smoke_dir" ] || { printf 'cuda-package-smoke: smoke directory must be new: %s\n' "$smoke_dir" >&2; exit 1; }
[ -d "$hf_home" ] && [ ! -L "$hf_home" ] || { printf 'cuda-package-smoke: pinned embedder cache must be a non-symlink directory\n' >&2; exit 1; }
python_wheel_abs="$(realpath -- "$python_wheel")"
npm_main_abs="$(realpath -- "$npm_main")"
napi_platform_abs="$(realpath -- "$napi_platform")"
cli_archive_abs="$(realpath -- "$cli_archive")"
model_cache_manifest_abs="$(realpath -- "$model_cache_manifest")"
reranker_cache_manifest_abs=''
if [ -n "$reranker_cache_manifest" ]; then
  reranker_cache_manifest_abs="$(realpath -- "$reranker_cache_manifest")"
fi
hf_home_abs="$(realpath -- "$hf_home")"
python3 - "$hf_home_abs" "$model_cache_manifest_abs" <<'PY'
import hashlib, json, sys
from pathlib import Path
root, manifest_path = map(Path, sys.argv[1:])
manifest = json.loads(manifest_path.read_text())
if set(manifest) != {"schema_version", "repository", "revision", "snapshot_relpath", "files"} or manifest["schema_version"] != "fathomdb.cuda-model-cache/v1":
    raise SystemExit("cuda-package-smoke: invalid retained model cache manifest")
snapshot = root / manifest["snapshot_relpath"]
actual = {str(path.relative_to(snapshot)): path for path in snapshot.rglob("*") if path.is_file()}
if set(actual) != set(manifest["files"]):
    raise SystemExit("cuda-package-smoke: HF seed inventory differs from retained manifest")
for name, expected in manifest["files"].items():
    path = actual[name]
    if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
        raise SystemExit(f"cuda-package-smoke: HF seed member differs: {name}")
PY
if [ -n "$reranker_cache_manifest_abs" ]; then
  python3 - "$reranker_cache_manifest_abs" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_bytes())
expected = {
    "schema_version": "fathomdb.reranker-cache/v1",
    "repository": "cross-encoder/ms-marco-TinyBERT-L2-v2",
    "revision": "81d1926f67cb8eee2c2be17ca9f793c7c3bd20cc",
    "snapshot_relpath": "fathomdb/reranker/0290849b0459",
    "files": {
        "config.json": "2144195e107cd7ea61556478e7add12986ebfbc3085f924fc0b90c2410604879",
        "tokenizer.json": "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
        "model.safetensors": "a0e7364ddf91ff7028f1102e1b91ac7a72e3db4061241bd84efe45c72c9af03a",
    },
}
canonical = json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"
if path.read_bytes() != canonical or value != expected:
    raise SystemExit("cuda-package-smoke: reranker cache manifest differs from the fixed TinyBERT seed")
PY
fi
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

write_cli_record() {
  local mode="$1" policy="$2" ordinal="$3" exit_code="$4" env_present="$5"
  python3 - "$smoke_dir" "$cli_archive_abs" "$mode" "$policy" "$ordinal" "$exit_code" "$env_present" <<'PY'
import hashlib, json, re, sys
from pathlib import Path
root, archive = Path(sys.argv[1]), Path(sys.argv[2])
mode, policy, ordinal, exit_code, env_present = sys.argv[3:]
match = re.fullmatch(r"fathomdb-([0-9]+\.[0-9]+\.[0-9]+)-x86_64-unknown-linux-gnu\.tar\.gz", archive.name)
if match is None:
    raise SystemExit("cuda-package-smoke: invalid CLI archive coordinate")
version = match.group(1)
stdout_name = f"{mode}-cli-stdout.json"
raw = (root / stdout_name).read_bytes()
doctor = json.loads(raw)
if raw != json.dumps(doctor, ensure_ascii=True, separators=(",", ":")).encode("ascii") + b"\n":
    raise SystemExit("cuda-package-smoke: doctor output is not canonical JSON")
archive_sha = hashlib.sha256(archive.read_bytes()).hexdigest()
record = {
    "schema_version": "fathomdb.cuda-package-cli-smoke/v2", "consumer": "cli",
    "archive_filename": archive.name, "archive_sha256": archive_sha,
    "target": "x86_64-unknown-linux-gnu",
    "argv": [f"/fathomdb-cli/fathomdb-{version}-x86_64-unknown-linux-gnu/fathomdb", "doctor", "gpu", "--json"],
    "requested_policy": policy, "requested_ordinal": None if ordinal == "null" else int(ordinal),
    "environment": {"FATHOMDB_EMBED_DEVICE": policy} if env_present == "true" else {},
    "isolation": {"database_opened": False, "model_loaded": False, "network": "none", "source_checkout_mounted": False},
    "evidence_provenance": "installed_candidate", "exit_code": int(exit_code),
    "doctor_output_filename": stdout_name, "doctor_output_sha256": hashlib.sha256(raw).hexdigest(),
    "status": doctor["status"], "effective_device": doctor["effective_device"], "reason": doctor["reason"],
}
(root / f"{mode}-cli.json").write_text(json.dumps(record, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
}

write_reranker_cli_doctor_record() {
  python3 - "$smoke_dir" "$cli_archive_abs" <<'PY'
import hashlib, json, re, sys
from pathlib import Path

root, archive = map(Path, sys.argv[1:])
match = re.fullmatch(r"fathomdb-([0-9]+\.[0-9]+\.[0-9]+)-x86_64-unknown-linux-gnu\.tar\.gz", archive.name)
if match is None:
    raise SystemExit("cuda-package-smoke: invalid CLI archive coordinate")
version = match.group(1)
raw = (root / "reranker-cli-doctor-stdout.json").read_bytes()
doctor = json.loads(raw)
if raw != json.dumps(doctor, ensure_ascii=True, separators=(",", ":")).encode("ascii") + b"\n":
    raise SystemExit("cuda-package-smoke: reranker doctor output is not canonical JSON")
record = {
    "schema_version": "fathomdb.cuda-reranker-cli-doctor/v1",
    "consumer": "cli",
    "archive_filename": archive.name,
    "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
    "target": "x86_64-unknown-linux-gnu",
    "argv": [f"/fathomdb-cli/fathomdb-{version}-x86_64-unknown-linux-gnu/fathomdb", "doctor", "reranker-gpu", "--json"],
    "requested_policy": "auto",
    "environment": {},
    "isolation": {"database_opened": False, "model_loaded": False, "network": "none", "source_checkout_mounted": False},
    "evidence_provenance": "installed_candidate",
    "exit_code": 0,
    "doctor_output_filename": "reranker-cli-doctor-stdout.json",
    "doctor_output_sha256": hashlib.sha256(raw).hexdigest(),
    "effective_device": doctor["effective_device"],
    "reason": doctor["reason"],
}
(root / "reranker-cli-doctor.json").write_text(json.dumps(record, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
}

# These commands mount only retained artifact bytes. The host checkout is
# never mounted; env -i and --network none prevent ambient package/download or
# device-selector inputs from satisfying a CPU-default smoke.
docker run --rm --network none \
  --mount "type=bind,src=$python_wheel_abs,dst=/input/fathomdb.whl,readonly" \
  --mount "type=bind,src=$hf_home_abs,dst=/fathomdb-hf,readonly" \
  "$CUDA_DRIVERLESS_PYTHON_IMAGE" \
  env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable HF_HOME=/fathomdb-hf XDG_CACHE_HOME=/fathomdb-product-cache sh -ceu '
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
  env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable HF_HOME=/fathomdb-hf XDG_CACHE_HOME=/fathomdb-product-cache sh -ceu '
    test ! -e /dev/nvidiactl
    mkdir /consumer && cd /consumer
    printf "%s\n" "{\"private\":true,\"type\":\"module\",\"dependencies\":{\"fathomdb\":\"file:/input/fathomdb.tgz\",\"fathomdb-linux-x64-gnu\":\"file:/input/fathomdb-linux-x64-gnu.tgz\"}}" > package.json
    npm install --offline --ignore-scripts --no-audit --no-fund
    node --input-type=module -e "import { Engine } from \"fathomdb\"; const e=await Engine.open(\"/tmp/cpu.fdb\",{useDefaultEmbedder:true}); await e.embed(\"CUDA package rehearsal installed N-API CPU smoke\"); await e.write([{kind:\"doc\",body:\"{}\",sourceId:\"cuda-package-cpu\"}]); await e.search(\"smoke\"); await e.close();"
  '
write_cpu napi

# CLI proof is intentionally diagnostic-only. A short-lived doctor command
# cannot truthfully supply a GPU PID observation, so compatible-GPU CLI and
# deterministic incompatible-classifier evidence remain PENDING_EXTERNAL.
smoke_dir_abs="$(realpath -- "$smoke_dir")"
for mode in cpu forced-cuda-unavailable; do
  policy=auto expected=0 ordinal=null env_present=false
  doctor_env=()
  if [ "$mode" != cpu ]; then
    policy=cuda:0 expected=65 ordinal=0 env_present=true
    doctor_env=(FATHOMDB_EMBED_DEVICE=cuda:0)
  fi
  set +e
  docker run --rm --network none \
    --mount "type=bind,src=$cli_archive_abs,dst=/input/fathomdb-cli.tar.gz,readonly" \
    --mount "type=bind,src=$smoke_dir_abs,dst=/evidence" \
    "$CUDA_DRIVERLESS_PYTHON_IMAGE" \
    # FATHOMDB_RERANK_DEVICE is absent under env -i: the product's unset=auto path is the evidence.
    env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable \
      "${doctor_env[@]}" sh -ceu '
        mkdir /fathomdb-cli
        tar -xzf /input/fathomdb-cli.tar.gz -C /fathomdb-cli
        binary="$(find /fathomdb-cli -mindepth 2 -maxdepth 2 -type f -name fathomdb -print -quit)"
        exec "$binary" doctor gpu --json
      ' > "$smoke_dir/$mode-cli-stdout.json"
  observed=$?
  set -e
  [ "$observed" -eq "$expected" ] || { printf 'cuda-package-smoke: CLI %s exit %s, expected %s\n' "$mode" "$observed" "$expected" >&2; exit 1; }
  write_cli_record "$mode" "$policy" "$ordinal" "$observed" "$env_present"
done

if [ -n "$reranker_cache_manifest_abs" ]; then
  set +e
  docker run --rm --network none \
    --mount "type=bind,src=$cli_archive_abs,dst=/input/fathomdb-cli.tar.gz,readonly" \
    "$CUDA_DRIVERLESS_PYTHON_IMAGE" \
    env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable \
      sh -ceu '
        mkdir /fathomdb-cli
        tar -xzf /input/fathomdb-cli.tar.gz -C /fathomdb-cli
        binary="$(find /fathomdb-cli -mindepth 2 -maxdepth 2 -type f -name fathomdb -print -quit)"
        exec "$binary" doctor reranker-gpu --json
      ' > "$smoke_dir/reranker-cli-doctor-stdout.json"
  observed=$?
  set -e
  [ "$observed" -eq 0 ] || { printf 'cuda-package-smoke: reranker CLI doctor exit %s, expected 0\n' "$observed" >&2; exit 1; }
  write_reranker_cli_doctor_record
fi

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
    env -i PATH=/opt/python/cp311-cp311/bin:/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable HF_HOME=/fathomdb-hf XDG_CACHE_HOME=/fathomdb-product-cache FATHOMDB_EMBED_DEVICE=cuda:0 /opt/python/cp311-cp311/bin/python -m pip install --no-deps /input/fathomdb.whl
    exec env -i PATH=/opt/python/cp311-cp311/bin:/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable HF_HOME=/fathomdb-hf XDG_CACHE_HOME=/fathomdb-product-cache FATHOMDB_EMBED_DEVICE=cuda:0 /opt/python/cp311-cp311/bin/python -c "from fathomdb import Engine; import tempfile; from pathlib import Path; d=tempfile.TemporaryDirectory(); e=Engine.open(str(Path(d.name)/\"gpu.fdb\"),use_default_embedder=True); e.embed(\"CUDA package rehearsal installed Python GPU smoke\"); e.close(); import time; time.sleep(20)"
  ')"
wait_for_gpu "$python_gpu" python

napi_gpu="$(docker run -d --gpus '"'"'device=0'"'"' --network none \
  --mount "type=bind,src=$npm_main_abs,dst=/input/fathomdb.tgz,readonly" \
  --mount "type=bind,src=$napi_platform_abs,dst=/input/fathomdb-linux-x64-gnu.tgz,readonly" \
  --mount "type=bind,src=$hf_home_abs,dst=/fathomdb-hf,readonly" \
  "$CUDA_DRIVERLESS_NODE_IMAGE" sh -ceu '
    mkdir /consumer && cd /consumer
    printf "%s\n" "{\"private\":true,\"type\":\"module\",\"dependencies\":{\"fathomdb\":\"file:/input/fathomdb.tgz\",\"fathomdb-linux-x64-gnu\":\"file:/input/fathomdb-linux-x64-gnu.tgz\"}}" > package.json
    env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable HF_HOME=/fathomdb-hf XDG_CACHE_HOME=/fathomdb-product-cache npm install --offline --ignore-scripts --no-audit --no-fund
    exec env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable HF_HOME=/fathomdb-hf XDG_CACHE_HOME=/fathomdb-product-cache FATHOMDB_EMBED_DEVICE=cuda:0 node --input-type=module -e "import { Engine } from \"fathomdb\"; const e=await Engine.open(\"/tmp/gpu.fdb\",{useDefaultEmbedder:true}); await e.embed(\"CUDA package rehearsal installed N-API GPU smoke\"); await e.close(); await new Promise(resolve=>setTimeout(resolve,20000));"
  ')"
wait_for_gpu "$napi_gpu" napi
