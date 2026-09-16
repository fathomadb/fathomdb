#!/usr/bin/env bash
# Build and smoke-test the Python wheel without importing from the checkout.
set -euo pipefail

usage() {
  echo "usage: $0 --python PYTHON --wheel-dir DIR --venv-dir DIR" >&2
  exit 2
}

python_bin=""
wheel_dir=""
venv_dir=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --python) [ "$#" -ge 2 ] || usage; python_bin="$2"; shift 2 ;;
    --wheel-dir) [ "$#" -ge 2 ] || usage; wheel_dir="$2"; shift 2 ;;
    --venv-dir) [ "$#" -ge 2 ] || usage; venv_dir="$2"; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$python_bin" ] && [ -n "$wheel_dir" ] && [ -n "$venv_dir" ] || usage
[ ! -e "$wheel_dir" ] || { echo "wheel directory must not already exist: $wheel_dir" >&2; exit 1; }
[ ! -e "$venv_dir" ] || { echo "venv directory must not already exist: $venv_dir" >&2; exit 1; }
[ "$wheel_dir" != "$venv_dir" ] || { echo "wheel and venv directories must differ" >&2; exit 1; }

repo="$(git rev-parse --show-toplevel)"
mkdir -p "$wheel_dir"
(
  cd "$repo/src/python"
  maturin build --release --out "$wheel_dir" \
    --features pyo3/extension-module,default-embedder -i "$python_bin"
)

wheel_manifest="$wheel_dir/.wheel-manifest"
find "$wheel_dir" -maxdepth 1 -type f -name '*.whl' -print >"$wheel_manifest"
mapfile -t wheels <"$wheel_manifest"
if [ "${#wheels[@]}" -ne 1 ]; then
  echo "expected exactly one wheel in $wheel_dir; found ${#wheels[@]}" >&2
  exit 1
fi

"$python_bin" -m venv "$venv_dir"
"$venv_dir/bin/python" -m pip install --no-index --no-deps "${wheels[0]}"
report="$wheel_dir/install-provenance.txt"
matrix_report="$wheel_dir/slice50-graph-evidence.json"
profile="$wheel_dir/frozen-evidence-profile.py"
cp "$repo/scripts/release/smoke/frozen-evidence-python.py" "$profile"
env -u PYTHONPATH -u VIRTUAL_ENV \
  FATHOMDB_VERIFY_REPORT="$report" \
  FATHOMDB_SLICE50_EVIDENCE_REPORT="$matrix_report" \
  FAKE_VENV="$venv_dir" \
  "$venv_dir/bin/python" "$profile"

mapfile -t provenance <"$report"
[ "${#provenance[@]}" -eq 4 ] || { echo "invalid wheel provenance report" >&2; exit 1; }
venv_real="$(cd "$venv_dir" && pwd -P)"
case "${provenance[0]}" in "$venv_real"/*) ;; *) echo "module escaped fresh venv: ${provenance[0]}" >&2; exit 1 ;; esac
case "${provenance[1]}" in "$venv_real"/*) ;; *) echo "native module escaped fresh venv: ${provenance[1]}" >&2; exit 1 ;; esac
[ "${provenance[2]}" = false ] || { echo "editable install is not release evidence" >&2; exit 1; }
[ "${provenance[3]}" = frozen-evidence-profile-v1 ] \
  || { echo "frozen evidence profile did not complete" >&2; exit 1; }
python3 - "$repo/scripts/release/slice50-evidence-matrix.py" "$matrix_report" <<'PY'
import importlib.util
import json
import sys

module_path, report_path = sys.argv[1:]
spec = importlib.util.spec_from_file_location("slice50_evidence_matrix", module_path)
assert spec is not None and spec.loader is not None
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
payload = json.load(open(report_path, encoding="utf-8"))
assert payload["schema_version"] == "fathomdb.slice50-graph-evidence/v1"
module.validate_rows(payload["rows"])
refusal = payload["schema_33_refusal"]
assert refusal["expected_schema"] == 33
assert refusal["supported_schema"] == 34
assert refusal["outcome"] == "typed_refusal"
assert refusal["before"] == refusal["after"]
PY

sha256sum "${wheels[0]}"
printf 'module=%s\nnative=%s\n' "${provenance[0]}" "${provenance[1]}"
