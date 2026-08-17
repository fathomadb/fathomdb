#!/usr/bin/env bash
# Build a non-publishing, fail-closed CUDA package-rehearsal evidence bundle.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

usage() {
  cat >&2 <<'USAGE'
usage: cuda-package-rehearsal.sh --candidate-sha SHA --route-receipt FILE --route-manifest FILE \
  --preflight-witness-dir DIR --build-input FILE --python-wheel FILE --npm-main FILE \
  --napi-platform FILE --smoke-dir DIR --output-dir DIR --workflow-ref REF --workflow-sha SHA \
  --run-id N --run-attempt N
USAGE
}

candidate_sha='' route_receipt='' route_manifest='' witness_dir='' build_input=''
python_wheel='' npm_main='' napi_platform='' smoke_dir='' output_dir=''
workflow_ref='' workflow_sha='' run_id='' run_attempt=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --candidate-sha) candidate_sha="$2" ;;
    --route-receipt) route_receipt="$2" ;;
    --route-manifest) route_manifest="$2" ;;
    --preflight-witness-dir) witness_dir="$2" ;;
    --build-input) build_input="$2" ;;
    --python-wheel) python_wheel="$2" ;;
    --npm-main) npm_main="$2" ;;
    --napi-platform) napi_platform="$2" ;;
    --smoke-dir) smoke_dir="$2" ;;
    --output-dir) output_dir="$2" ;;
    --workflow-ref) workflow_ref="$2" ;;
    --workflow-sha) workflow_sha="$2" ;;
    --run-id) run_id="$2" ;;
    --run-attempt) run_attempt="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done

for value in candidate_sha route_receipt route_manifest witness_dir build_input python_wheel npm_main napi_platform smoke_dir output_dir workflow_ref workflow_sha run_id run_attempt; do
  [ -n "${!value}" ] || { printf 'cuda-package-rehearsal: missing --%s\n' "${value//_/-}" >&2; exit 2; }
done
if [ -e "$output_dir" ]; then
  printf 'cuda-package-rehearsal: output directory must be new: %s\n' "$output_dir" >&2
  exit 1
fi

# A real output cannot exist until both independently verified Slice 10 inputs
# bind this candidate and run. The receipt verifier intentionally accepts only
# the checked-out main-owned control-plane manifest path.
python3 "$SCRIPT_DIR/verify-cuda-unmerged-receipt.py" \
  --receipt "$route_receipt" \
  --manifest "$route_manifest" \
  --candidate-sha "$candidate_sha" \
  --workflow-ref "$workflow_ref" \
  --workflow-sha "$workflow_sha" \
  --run-id "$run_id" \
  --run-attempt "$run_attempt"
python3 "$SCRIPT_DIR/verify-cuda-preflight-witness.py" \
  --witness-dir "$witness_dir" \
  --candidate-sha "$candidate_sha"

for path in "$build_input" "$python_wheel" "$npm_main" "$napi_platform"; do
  [ -f "$path" ] && [ ! -L "$path" ] || { printf 'cuda-package-rehearsal: input must be a regular non-symlink file: %s\n' "$path" >&2; exit 1; }
done
[ -d "$smoke_dir" ] && [ ! -L "$smoke_dir" ] || { printf 'cuda-package-rehearsal: smoke directory must be a non-symlink directory\n' >&2; exit 1; }

mkdir -p "$output_dir/packages" "$output_dir/smoke"
cp -- "$route_receipt" "$output_dir/route-receipt.json"
cp -- "$witness_dir/cuda-preflight-witness.json" "$output_dir/preflight-witness.json"
cp -- "$build_input" "$output_dir/build-input.json"
cp -- "$python_wheel" "$output_dir/packages/$(basename "$python_wheel")"
cp -- "$npm_main" "$output_dir/packages/$(basename "$npm_main")"
cp -- "$napi_platform" "$output_dir/packages/$(basename "$napi_platform")"
for name in cpu-python.json cpu-napi.json gpu-python.json gpu-napi.json; do
  [ -f "$smoke_dir/$name" ] && [ ! -L "$smoke_dir/$name" ] || { printf 'cuda-package-rehearsal: required smoke evidence absent or symlinked: %s\n' "$name" >&2; exit 1; }
  cp -- "$smoke_dir/$name" "$output_dir/smoke/$name"
done

python3 - "$output_dir" "$candidate_sha" "$witness_dir/cuda-preflight-witness.json" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
candidate = sys.argv[2]
witness = Path(sys.argv[3])
if witness.is_symlink() or not witness.is_file():
    raise SystemExit("cuda-package-rehearsal: preflight witness is absent or symlinked")
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
build_input = json.loads((root / "build-input.json").read_bytes())
manifest = {
    "schema_version": "fathomdb.cuda-package-rehearsal/v1",
    "candidate_sha": candidate,
    "route_receipt_sha256": digest(root / "route-receipt.json"),
    "preflight_witness_sha256": digest(witness),
    "build_input": build_input,
    "packages": {path.name: digest(path) for path in sorted((root / "packages").iterdir())},
    "smoke_evidence_sha256": {path.name: digest(path) for path in sorted((root / "smoke").iterdir())},
}
(root / "cuda-package-rehearsal.json").write_bytes(
    json.dumps(manifest, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"
)
PY
python3 "$SCRIPT_DIR/verify-cuda-package-rehearsal.py" --rehearsal-dir "$output_dir" --candidate-sha "$candidate_sha"
printf 'cuda-package-rehearsal: created %s\n' "$output_dir"
