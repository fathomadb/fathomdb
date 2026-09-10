#!/usr/bin/env bash
# Run GLOBAL-01 from an installed candidate wheel in a disposable clean clone.
set -euo pipefail

usage() {
  printf 'usage: %s --wheel-dir DIR --config FILE --receipt FILE\n' "$0" >&2
}

wheel_dir='' config='' receipt=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --wheel-dir) wheel_dir="$2" ;;
    --config) config="$2" ;;
    --receipt) receipt="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done
[ -d "$wheel_dir" ] && [ -f "$config" ] && [ -n "$receipt" ] || { usage; exit 2; }
repo_root="$(git rev-parse --show-toplevel)"
candidate_sha="$(git rev-parse HEAD)"
tracked_status="$(git status --porcelain=v1 --untracked-files=no)"
[ -z "$tracked_status" ] \
  || { printf 'slice75-global-native: tracked tree must be clean\n' >&2; exit 1; }
shopt -s nullglob
wheels=("$wheel_dir"/*.whl)
shopt -u nullglob
[ "${#wheels[@]}" -eq 1 ] || { printf 'slice75-global-native: expected one wheel\n' >&2; exit 1; }
fathomdb_bin="$(python3 - "$config" <<'PY'
import json, sys
value=json.load(open(sys.argv[1], encoding="utf-8"))
print(value["fathomdb_bin"])
PY
)"
[ -x "$repo_root/$fathomdb_bin" ] \
  || { printf 'slice75-global-native: configured candidate CLI is unavailable\n' >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
git clone --quiet --no-hardlinks --shared "$repo_root" "$work/repository"
mkdir -p "$work/repository/$(dirname "$fathomdb_bin")"
cp "$repo_root/$fathomdb_bin" "$work/repository/$fathomdb_bin"
python3 -m venv "$work/venv"
"$work/venv/bin/python" -m pip install --no-index "${wheels[0]}"
config_relative="$(realpath --relative-to="$repo_root" "$config")"
(
  cd "$work/repository"
  "$work/venv/bin/python" -m experiments.measurement_classification run-native \
    --repository-root . --config "$config_relative"
)
run_dir="$(find "$work/repository/experiments/runs" -mindepth 1 -maxdepth 1 -type d \
  -name 'measurement-classification-native-search-*' -printf '%T@ %p\n' \
  | sort -nr | awk 'NR == 1 {sub(/^[^ ]+ /, ""); print}')"
[ -n "$run_dir" ] || { printf 'slice75-global-native: native run receipt is missing\n' >&2; exit 1; }
python3 - "$run_dir" "$candidate_sha" "$work/venv" "$receipt" <<'PY'
import hashlib
import json
import pathlib
import sys

run_dir = pathlib.Path(sys.argv[1])
candidate_sha = sys.argv[2]
venv = pathlib.Path(sys.argv[3]).resolve()
receipt = pathlib.Path(sys.argv[4])
metrics = json.loads((run_dir / "metrics.json").read_text())
classification = json.loads((run_dir / "measurement-classification.v2.json").read_text())
attestation = json.loads((run_dir / "runtime-attestation.v1.json").read_text())
result = json.loads((run_dir / "search-result.v1.json").read_text())
retrieval = metrics["retrieval"]
if (
    metrics["state"] != "complete"
    or retrieval["call_count"] != 1
    or retrieval["recall_at_3"] != 1.0
    or retrieval["reciprocal_rank"] != 1.0
    or result["expected_rank"] != 1
    or classification["outcome"] != "complete"
):
    raise SystemExit("slice75-global-native: GLOBAL-01 result did not meet the sealed predicates")
native = pathlib.Path(attestation["native_extension"]["path"]).resolve()
if venv not in native.parents:
    raise SystemExit("slice75-global-native: Python resolved outside the installed candidate environment")
payload = {
    "schema_version": "fathomdb.slice75-global-native/v1",
    "candidate_sha": candidate_sha,
    "run_id": classification["run_id"],
    "measurement_layer": "data_plane",
    "engine_search_calls": 1,
    "expected_rank": 1,
    "recall_at_3": 1,
    "reciprocal_rank": 1,
    "spend_usd": 0,
    "answer_quality_claim": False,
    "source_fallback": False,
    "native_extension": str(native),
    "native_extension_sha256": hashlib.sha256(native.read_bytes()).hexdigest(),
}
receipt.parent.mkdir(parents=True, exist_ok=True)
receipt.write_text(json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
printf 'slice75-global-native: pass; receipt at %s\n' "$receipt"
