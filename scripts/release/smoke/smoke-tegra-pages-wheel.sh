#!/usr/bin/env bash
# Install and exercise the exact public Tegra wheel from the interim Pages index.
# A sole --index-url is intentional: never accept --extra-index-url here, since
# pip would merge candidates from multiple sources rather than prioritize Pages.
set -euo pipefail

usage() {
  printf 'usage: %s <version+tegra> <https://fathomadb.github.io/fathomdb/tegra/simple/> <wheel-sha256> <candidate-sha> <pages-deployment-run> <new-evidence-dir>\n' "$0" >&2
  printf '       %s --local-wheel FILE <version+tegra> <wheel-sha256> <candidate-sha> <new-evidence-dir>\n' "$0" >&2
}

if [ "$#" -ne 6 ]; then
  usage
  exit 2
fi

LOCAL_WHEEL=''
if [ "$1" = --local-wheel ]; then
  LOCAL_WHEEL="$2"
  VERSION="$3"
  EXPECTED_SHA256="$4"
  CANDIDATE_SHA="$5"
  EVIDENCE_DIR="$6"
  INDEX_URL=''
  PAGES_DEPLOYMENT_RUN=''
  [ -f "$LOCAL_WHEEL" ] || { printf 'smoke-tegra-pages-wheel: local wheel is absent: %s\n' "$LOCAL_WHEEL" >&2; exit 1; }
  LOCAL_WHEEL="$(realpath -- "$LOCAL_WHEEL")"
else
  VERSION="$1"
  INDEX_URL="$2"
  EXPECTED_SHA256="$3"
  CANDIDATE_SHA="$4"
  PAGES_DEPLOYMENT_RUN="$5"
  EVIDENCE_DIR="$6"
fi
EXPECTED_INDEX_URL='https://fathomadb.github.io/fathomdb/tegra/simple/'

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+\+tegra$ ]]; then
  printf 'smoke-tegra-pages-wheel: invalid version %q; expected X.Y.Z+tegra\n' "$VERSION" >&2
  exit 2
fi
if [ -z "$LOCAL_WHEEL" ] && [ "$INDEX_URL" != "$EXPECTED_INDEX_URL" ]; then
  printf 'smoke-tegra-pages-wheel: invalid index %q; expected the authorized first-party index\n' "$INDEX_URL" >&2
  exit 2
fi
if [[ ! "$EXPECTED_SHA256" =~ ^[0-9a-f]{64}$ ]]; then
  printf 'smoke-tegra-pages-wheel: invalid expected SHA-256\n' >&2
  exit 2
fi
if [[ ! "$CANDIDATE_SHA" =~ ^[0-9a-f]{40}$ ]]; then
  printf 'smoke-tegra-pages-wheel: invalid candidate SHA\n' >&2
  exit 2
fi
if [ -z "$LOCAL_WHEEL" ] && [[ ! "$PAGES_DEPLOYMENT_RUN" =~ ^[0-9]+$ ]]; then
  printf 'smoke-tegra-pages-wheel: invalid Pages deployment run\n' >&2
  exit 2
fi
HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
if [ "$HOST_OS" != Linux ] || [ "$HOST_ARCH" != aarch64 ]; then
  printf 'smoke-tegra-pages-wheel: expected a Linux aarch64 Jetson host\n' >&2
  exit 1
fi
if ! grep -aq 'nvidia,tegra' /proc/device-tree/compatible 2>/dev/null \
  && [ ! -e /etc/nv_tegra_release ]; then
  printf 'smoke-tegra-pages-wheel: classic Tegra platform signal is absent\n' >&2
  exit 1
fi
NVIDIA_SMI=''
for candidate in /usr/bin/nvidia-smi /usr/sbin/nvidia-smi /usr/local/bin/nvidia-smi; do
  if [ -x "$candidate" ]; then
    NVIDIA_SMI="$candidate"
    break
  fi
done
if [ -z "$NVIDIA_SMI" ]; then
  printf 'smoke-tegra-pages-wheel: no approved nvidia-smi path is executable\n' >&2
  exit 1
fi
GPU_NAME="$(timeout 2s "$NVIDIA_SMI" --query-gpu=name --format=csv,noheader)"
case "$GPU_NAME" in
  *nvgpu*) ;;
  *)
    printf 'smoke-tegra-pages-wheel: host is not a confirmed classic Tegra CUDA device: %s\n' "$GPU_NAME" >&2
    exit 1
    ;;
esac

WORK="$(mktemp -d)"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT
if [ -e "$EVIDENCE_DIR" ]; then
  printf 'smoke-tegra-pages-wheel: evidence path must be new: %s\n' "$EVIDENCE_DIR" >&2
  exit 1
fi
mkdir -p "$EVIDENCE_DIR"

DOWNLOADS="$WORK/downloads"
mkdir -p "$DOWNLOADS"
python3 -m venv "$WORK/venv"
# shellcheck source=/dev/null
. "$WORK/venv/bin/activate"
pip_clean() {
  env -u PIP_CONFIG_FILE -u PIP_INDEX_URL -u PIP_EXTRA_INDEX_URL -u PIP_FIND_LINKS \
    -u PIP_NO_INDEX -u PIP_NO_BINARY -u PIP_ONLY_BINARY -u PYTHONPATH \
    PIP_CONFIG_FILE=/dev/null python3 -m pip --isolated "$@"
}
if [ -n "$LOCAL_WHEEL" ]; then
  cp -- "$LOCAL_WHEEL" "$DOWNLOADS/"
else
pip_clean download --quiet --index-url "$INDEX_URL" --only-binary=:all: --no-cache-dir --no-deps \
  --dest "$DOWNLOADS" "fathomdb==${VERSION}"
fi

WHEEL_LIST="$WORK/wheels.txt"
find "$DOWNLOADS" -maxdepth 1 -type f \
  -name "fathomdb-${VERSION}-*-linux_aarch64.whl" -print > "$WHEEL_LIST"
mapfile -t WHEELS < "$WHEEL_LIST"
if [ "${#WHEELS[@]}" -ne 1 ]; then
  printf 'smoke-tegra-pages-wheel: expected exactly one exact Tegra wheel, found %s\n' "${#WHEELS[@]}" >&2
  exit 1
fi
WHEEL="${WHEELS[0]}"
ACTUAL_SHA256="$(sha256sum "$WHEEL" | awk '{print $1}')"
if [ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]; then
  printf 'smoke-tegra-pages-wheel: downloaded wheel SHA-256 differs from retained evidence\n' >&2
  exit 1
fi
pip_clean install --quiet --no-index --no-deps "$WHEEL"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CACHE_ROOT="${FATHOMDB_TEGRA_HF_HOME:-$HOME/.cache/huggingface}"
if [ ! -d "$CACHE_ROOT" ]; then
  printf 'smoke-tegra-pages-wheel: offline model cache is absent: %s\n' "$CACHE_ROOT" >&2
  exit 1
fi

export HF_HOME="$CACHE_ROOT"
export HF_HUB_OFFLINE=1
export TRANSFORMERS_OFFLINE=1
export FATHOMDB_EMBED_DEVICE=cuda:0
export FATHOMDB_GPU_ALLOCATION_WITNESS=1
GPU_INFO="$(timeout 2s "$NVIDIA_SMI" --query-gpu=uuid,name,driver_version --format=csv,noheader)"
export EVIDENCE_DIR VERSION INDEX_URL EXPECTED_SHA256 ACTUAL_SHA256 CANDIDATE_SHA PAGES_DEPLOYMENT_RUN GPU_INFO LOCAL_WHEEL
unset PYTHONPATH
for policy in cpu auto cuda:0; do
timeout --kill-after=5s 120s env FATHOMDB_EMBED_DEVICE="$policy" FATHOMDB_RERANK_DEVICE="$policy" \
  python3 - "$WORK/smoke-$policy.sqlite" <<'PY'
import json
import os
import sys
from dataclasses import asdict
from importlib.metadata import version
from pathlib import Path

from fathomdb import Engine, embed_batch_cls, rerank
import math

def _witness_for_process(witness):
    if witness is not None:
        pid = witness.process_id
        if not isinstance(pid, int) or isinstance(pid, bool) or pid != os.getpid():
            raise RuntimeError("allocation witness does not belong to the consumer process")
    return witness

if version("fathomdb") != os.environ["VERSION"]:
    raise SystemExit("installed distribution version differs from requested Tegra version")

evidence = Path(os.environ["EVIDENCE_DIR"])
engine = Engine.open(sys.argv[1], use_default_embedder=True)
try:
    report = engine.open_report()
    resolution = report.embedder_device_resolution
    witness = _witness_for_process(report.embedder_gpu_allocation_witness)
    policy = os.environ["FATHOMDB_EMBED_DEVICE"]
    expected_device = "cpu" if policy == "cpu" else "cuda"
    if resolution is None or resolution.effective_device.kind != expected_device:
        raise SystemExit(f"{policy} did not produce expected {expected_device} embedding")
    reranker = report.reranker_device_resolution
    if reranker is None or reranker.effective_device.kind != expected_device:
        raise SystemExit(f"{policy} did not produce expected {expected_device} reranker")
    if expected_device == "cuda" and witness is None:
        raise SystemExit("forced CUDA did not produce an effective CUDA device and allocation witness")
    engine.write([{"kind": "doc", "body": "{}", "source_id": "smoke:tegra-pages-wheel"}])
    engine.search("smoke")
    vector = engine.embed("fathomdb sync allocator repair experiment")
    batch = embed_batch_cls([f"batch32 item {k} about pools" for k in range(32)])
    if len(vector) != 384 or len(batch) != 32 or any(len(v) != 384 or not all(math.isfinite(x) for x in v) for v in [vector, *batch]):
        raise SystemExit("installed shipping embedding single/batch32 output is invalid")
    ranked = rerank("Which device shares DRAM?", [
        {"id": 1, "body": "The Jetson AGX Orin shares DRAM between CPU and GPU.", "score": 0.5},
        {"id": 2, "body": "Stream-ordered allocation uses a per-device memory pool.", "score": 0.4},
    ], 2)
    if len(ranked) != 2 or any(r["ce_score"] is None or not math.isfinite(r["ce_score"]) for r in ranked):
        raise SystemExit("installed shipping reranker forward is missing")
    if witness is not None:
        (evidence / "tegra-gpu-allocation-witness.json").write_text(
        json.dumps(asdict(witness), ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n",
        encoding="ascii",
    )
    record_name = "tegra-pages-smoke.json" if policy == "cuda:0" else f"tegra-pages-smoke-{policy}.json"
    (evidence / record_name).write_text(
        json.dumps(
            {
                "schema_version": "fathomdb.tegra-pages-installed-smoke/v1",
                "source": "local" if os.environ["LOCAL_WHEEL"] else "pages",
                "local_wheel": os.environ["LOCAL_WHEEL"] or None,
                "distribution": "fathomdb",
                "version": os.environ["VERSION"],
                "index_url": os.environ["INDEX_URL"] or None,
                "expected_wheel_sha256": os.environ["EXPECTED_SHA256"],
                "actual_wheel_sha256": os.environ["ACTUAL_SHA256"],
                "candidate_sha": os.environ["CANDIDATE_SHA"],
                "pages_deployment_run": os.environ["PAGES_DEPLOYMENT_RUN"] or None,
                "host": {"system": os.uname().sysname, "machine": os.uname().machine},
                "python_version": sys.version.split()[0],
                "nvidia_smi": os.environ["GPU_INFO"],
                "policy": policy,
                "effective_device": resolution.effective_device.kind,
                "reranker_effective_device": reranker.effective_device.kind,
                "selected_cuda_uuid": resolution.selected_cuda_uuid,
                "lifecycle": "open/write/search/close",
            },
            ensure_ascii=True,
            separators=(",", ":"),
            sort_keys=True,
        ) + "\n",
        encoding="ascii",
    )
finally:
    import signal
    signal.signal(signal.SIGALRM, lambda *_: os._exit(124))
    signal.alarm(5)
    engine.close()
PY
done

PATH="/usr/sbin:$PATH" python3 "$REPO_ROOT/scripts/release/verify-tegra-gpu-witness.py" \
  --witness "$EVIDENCE_DIR/tegra-gpu-allocation-witness.json" --nvidia-smi
printf 'smoke-tegra-pages-wheel: ok — %s selected artifact passed installed CPU/auto/CUDA lifecycle and CUDA witness\n' \
  "$VERSION"
