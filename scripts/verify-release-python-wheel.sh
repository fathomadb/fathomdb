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
env -u PYTHONPATH -u VIRTUAL_ENV \
  FATHOMDB_VERIFY_REPORT="$report" \
  FAKE_VENV="$venv_dir" \
  "$venv_dir/bin/python" - <<'PY'
import importlib.metadata
import json
import os
import pathlib
import tempfile

import fathomdb
from fathomdb import _fathomdb

module_path = pathlib.Path(fathomdb.__file__).resolve()
native_path = pathlib.Path(_fathomdb.__file__).resolve()
editable = False
dist = importlib.metadata.distribution("fathomdb")
direct_url = next(
    (
        pathlib.Path(dist.locate_file(item))
        for item in (dist.files or ())
        if item.name == "direct_url.json" and any(part.endswith(".dist-info") for part in item.parts)
    ),
    None,
)
if direct_url is not None and direct_url.exists():
    data = json.loads(direct_url.read_text(encoding="utf-8"))
    editable = bool(data.get("dir_info", {}).get("editable"))

with tempfile.TemporaryDirectory() as root:
    db = pathlib.Path(root) / "wheel-smoke.sqlite"
    engine = fathomdb.Engine.open(str(db), use_default_embedder=False)
    try:
        engine.write([{
            "kind": "doc",
            "body": "durable wheel provenance smoke",
            "source_id": "slice7-wheel-smoke",
        }])
        hits = engine.search("durable wheel").results
        assert any(hit.body == "durable wheel provenance smoke" for hit in hits)
    finally:
        engine.close()

pathlib.Path(os.environ["FATHOMDB_VERIFY_REPORT"]).write_text(
    f"{module_path}\n{native_path}\n{str(editable).lower()}\n",
    encoding="utf-8",
)
print("wheel smoke: ok")
PY

mapfile -t provenance <"$report"
[ "${#provenance[@]}" -eq 3 ] || { echo "invalid wheel provenance report" >&2; exit 1; }
venv_real="$(cd "$venv_dir" && pwd -P)"
case "${provenance[0]}" in "$venv_real"/*) ;; *) echo "module escaped fresh venv: ${provenance[0]}" >&2; exit 1 ;; esac
case "${provenance[1]}" in "$venv_real"/*) ;; *) echo "native module escaped fresh venv: ${provenance[1]}" >&2; exit 1 ;; esac
[ "${provenance[2]}" = false ] || { echo "editable install is not release evidence" >&2; exit 1; }

sha256sum "${wheels[0]}"
printf 'module=%s\nnative=%s\n' "${provenance[0]}" "${provenance[1]}"
