#!/usr/bin/env bash
# Reuse sealed CUDA package bytes for installed CPU/CUDA embed+rerank smoke.
set -euo pipefail

usage() {
  printf 'usage: %s --candidate-sha SHA --packages DIR --witness DIR --hf-home DIR --output DIR\n' "$0" >&2
}

candidate_sha='' packages='' witness='' hf_home='' output=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --candidate-sha) candidate_sha="$2" ;;
    --packages) packages="$2" ;;
    --witness) witness="$2" ;;
    --hf-home) hf_home="$2" ;;
    --output) output="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done
[ -n "$candidate_sha" ] && [ -d "$packages" ] && [ -d "$witness" ] \
  && [ -d "$hf_home" ] && [ -n "$output" ] \
  || { usage; exit 2; }
[[ "$candidate_sha" =~ ^[0-9a-f]{40}$ ]] \
  || { printf 'slice75-cuda-package-smoke: invalid candidate SHA\n' >&2; exit 2; }
head_sha="$(git rev-parse HEAD)"
[ "$head_sha" = "$candidate_sha" ] \
  || { printf 'slice75-cuda-package-smoke: candidate SHA differs from HEAD\n' >&2; exit 1; }
[ ! -e "$output" ] || { printf 'slice75-cuda-package-smoke: output must be new\n' >&2; exit 1; }
python3 scripts/release/verify-cuda-preflight-witness.py \
  --witness-dir "$witness" --candidate-sha "$candidate_sha"

shopt -s nullglob
wheels=("$packages"/*.whl)
cli_archives=("$packages"/fathomdb-*-x86_64-unknown-linux-gnu.tar.gz)
platform_packages=("$packages"/fathomdb-linux-x64-gnu-*.tgz)
all_npm_packages=("$packages"/fathomdb-*.tgz)
shopt -u nullglob
main_packages=()
for package in "${all_npm_packages[@]}"; do
  case "$(basename "$package")" in
    fathomdb-linux-x64-gnu-*.tgz) ;;
    *) main_packages+=("$package") ;;
  esac
done
for count in "${#wheels[@]}" "${#cli_archives[@]}" "${#platform_packages[@]}" "${#main_packages[@]}"; do
  [ "$count" -eq 1 ] || { printf 'slice75-cuda-package-smoke: package inventory is not unique\n' >&2; exit 1; }
done
reranker_cache_root="${FATHOMDB_CUDA_PREFLIGHT_RERANKER_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}}"
bash scripts/release/cuda-package-rehearsal-smoke.sh \
  --python-wheel "${wheels[0]}" \
  --npm-main "${main_packages[0]}" \
  --napi-platform "${platform_packages[0]}" \
  --cli-archive "${cli_archives[0]}" \
  --model-cache-manifest "$witness/model-cache-manifest.json" \
  --reranker-cache-manifest "$witness/reranker-cache-manifest.json" \
  --reranker-cache-root "$reranker_cache_root" \
  --hf-home "$hf_home" \
  --smoke-dir "$output"
python3 - "$output" "$candidate_sha" "${wheels[0]}" "${main_packages[0]}" \
  "${platform_packages[0]}" "${cli_archives[0]}" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
sha = sys.argv[2]
artifacts = [pathlib.Path(value) for value in sys.argv[3:]]
cpu = [json.loads((root / f"cpu-{consumer}.json").read_text()) for consumer in ("python", "napi")]
gpu = [json.loads((root / f"gpu-{consumer}.json").read_text()) for consumer in ("python", "napi")]
if any(value["outcome"] != "passed" for value in cpu + gpu):
    raise SystemExit("slice75-cuda-package-smoke: a package consumer did not pass")
if any(value["embed_model_forwards"] < 1 or value["rerank_model_forwards"] < 1 for value in gpu):
    raise SystemExit("slice75-cuda-package-smoke: positive CUDA model forwards are missing")
names = {value["device_name"] for value in gpu}
if len(names) != 1 or not any("RTX 3090" in name for name in names):
    raise SystemExit(f"slice75-cuda-package-smoke: expected RTX 3090, found {sorted(names)}")
for name in ("cpu-cli.json", "forced-cuda-unavailable-cli.json", "reranker-cli-doctor.json"):
    if not (root / name).is_file():
        raise SystemExit(f"slice75-cuda-package-smoke: missing CLI doctor record {name}")
payload = {
    "schema_version": "fathomdb.slice75-cuda-package/v1",
    "candidate_sha": sha,
    "outcome": "pass",
    "gpu": next(iter(names)),
    "consumers": ["python", "napi", "cli"],
    "policies": ["cpu", "auto", "cuda"],
    "cli_doctors": ["gpu", "reranker-gpu"],
    "python_napi_embed_model_forwards": 2,
    "python_napi_rerank_model_forwards": 2,
    "artifacts": {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in artifacts
    },
}
(root / "slice75-receipt.json").write_text(
    json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n"
)
PY
printf 'slice75-cuda-package-smoke: pass; receipt at %s/slice75-receipt.json\n' "$output"
