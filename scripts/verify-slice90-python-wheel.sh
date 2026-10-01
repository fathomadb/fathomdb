#!/usr/bin/env bash
# Build from this checkout, then prove and exercise the installed candidate wheel.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <expected-full-commit-sha>" >&2
  exit 2
fi

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
actual_commit="$(git -C "$repo_dir" rev-parse HEAD)"
if [[ "$actual_commit" != "$1" ]]; then
  echo "candidate checkout is $actual_commit, expected $1" >&2
  exit 1
fi
source_status="$(git -C "$repo_dir" status --porcelain -- src/python/fathomdb/engine.py src/rust/crates/fathomdb-py/src/lib.rs)"
if [[ -n "$source_status" ]]; then
  echo "candidate Python or PyO3 source is not committed" >&2
  exit 1
fi

proof_dir="$(mktemp -d /tmp/slice90-python-wheel.XXXXXX)"
mkdir -p "$proof_dir/wheel"
(
  cd "$repo_dir/src/python"
  maturin build --offline --interpreter python3 --out "$proof_dir/wheel"
)
wheel_path="$(find "$proof_dir/wheel" -maxdepth 1 -name '*.whl' -type f -print -quit)"
if [[ -z "$wheel_path" ]]; then
  echo "candidate wheel was not produced" >&2
  exit 1
fi

python3 -m venv --system-site-packages "$proof_dir/consumer"
"$proof_dir/consumer/bin/python" -I -m pip install --no-index --no-deps "$wheel_path"
cp "$repo_dir/src/python/tests/test_slice90_engine_config.py" "$proof_dir/test_slice90_engine_config.py"

env -u PYTHONPATH "$proof_dir/consumer/bin/python" -I - "$repo_dir" "$wheel_path" "$proof_dir" "$actual_commit" <<'PY' | tee "$proof_dir/provenance.txt"
import hashlib
import inspect
import pathlib
import sys
import zipfile

repo = pathlib.Path(sys.argv[1])
wheel = pathlib.Path(sys.argv[2])
proof = pathlib.Path(sys.argv[3])
commit = sys.argv[4]
import fathomdb
import fathomdb._fathomdb as native
from fathomdb import engine as wrapper

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

with zipfile.ZipFile(wheel) as archive:
    wheel_engine = archive.read("fathomdb/engine.py")
    native_name = next(name for name in archive.namelist() if name.startswith("fathomdb/_fathomdb.") and name.endswith(".so"))
    wheel_native = archive.read(native_name)
source_engine = (repo / "src/python/fathomdb/engine.py").read_bytes()
if wheel_engine != source_engine or b"config=native_config" not in wheel_engine:
    raise SystemExit("wheel engine.py does not match this candidate's forwarding source")
if pathlib.Path(wrapper.__file__).read_bytes() != wheel_engine:
    raise SystemExit("installed engine.py does not match the candidate wheel")
if pathlib.Path(native.__file__).read_bytes() != wheel_native:
    raise SystemExit("installed native module does not match the candidate wheel")
if not pathlib.Path(wrapper.__file__).is_relative_to(proof / "consumer"):
    raise SystemExit("Python imported the wrapper outside the isolated consumer")
if not pathlib.Path(native.__file__).is_relative_to(proof / "consumer"):
    raise SystemExit("Python imported the native module outside the isolated consumer")
if "config" not in inspect.signature(native.Engine.open).parameters:
    raise SystemExit("native Engine.open lacks the configured-open parameter")

print(f"candidate_commit={commit}")
print(f"wheel_path={wheel}")
print(f"wheel_sha256={sha256(wheel.read_bytes())}")
print(f"source_engine_sha256={sha256(source_engine)}")
print(f"wheel_engine_sha256={sha256(wheel_engine)}")
print(f"installed_engine_path={wrapper.__file__}")
print(f"installed_engine_sha256={sha256(pathlib.Path(wrapper.__file__).read_bytes())}")
print(f"wheel_native_sha256={sha256(wheel_native)}")
print(f"installed_native_path={native.__file__}")
print(f"installed_native_sha256={sha256(pathlib.Path(native.__file__).read_bytes())}")
PY

(
  cd "$proof_dir"
  env -u PYTHONPATH "$proof_dir/consumer/bin/python" -I -m pytest -q \
    "$proof_dir/test_slice90_engine_config.py" | tee "$proof_dir/pytest.txt"
)
echo "proof_dir=$proof_dir"
