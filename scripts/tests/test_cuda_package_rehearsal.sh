#!/usr/bin/env bash
# Fail-closed tests for the retained CUDA package-rehearsal bundle.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERIFIER="$REPO_ROOT/scripts/release/verify-cuda-package-rehearsal.py"
SCHEMA="$REPO_ROOT/scripts/release/cuda-package-rehearsal.schema.json"
REHEARSE="$REPO_ROOT/scripts/release/cuda-package-rehearsal.sh"
CANDIDATE="0123456789abcdef0123456789abcdef01234567"
OTHER_CANDIDATE="89abcdef0123456789abcdef0123456789abcdef"
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

fail() { printf 'FAIL  %s\n' "$*" >&2; exit 1; }
pass() { printf 'PASS  %s\n' "$*"; }

canonical_json() {
  python3 - "$@" <<'PY'
import json
import sys

value = json.load(sys.stdin)
sys.stdout.write(json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
}

make_valid_bundle() {
  local root="$1"
  mkdir -p "$root/packages" "$root/smoke"
  printf 'wheel bytes\n' > "$root/packages/fathomdb-0.8.23-cp311-cp311-manylinux_2_28_x86_64.whl"
  printf 'thin npm bytes\n' > "$root/packages/fathomdb-0.8.23.tgz"
  printf 'napi bytes\n' > "$root/packages/fathomdb-linux-x64-gnu-0.8.23.tgz"
  cat > "$root/build-input.json" <<EOF
{"candidate_sha":"$CANDIDATE","napi_features":["default-embedder","embed-cuda"],"python_features":["embed-cuda","pyo3/extension-module"],"rerank_cuda":false,"schema_version":"fathomdb.cuda-package-build-input/v1"}
EOF
  cat > "$root/route-receipt.json" <<EOF
{"candidate_sha":"$CANDIDATE","schema_version":"fathomdb.cuda-unmerged-route-receipt/v1"}
EOF
  mkdir -p "$root/preflight-witness"
  PYTHONPATH="$REPO_ROOT/scripts/tests" python3 - "$root/preflight-witness" "$REPO_ROOT" <<'PY'
from pathlib import Path
import sys
from cuda_preflight_v2_fixture import make_valid

make_valid(Path(sys.argv[1]), Path(sys.argv[2]))
PY
  for consumer in python napi; do
    cat > "$root/smoke/cpu-$consumer.json" <<EOF
{"consumer":"$consumer","environment":"env -i","gpu_nodes_visible":false,"network":"none","outcome":"passed","schema_version":"fathomdb.cuda-package-cpu-smoke/v1","source_imported":false}
EOF
    cat > "$root/smoke/gpu-$consumer.json" <<EOF
{"consumer":"$consumer","device_name":"NVIDIA test","driver_version":"999.99","gpu_uuid":"GPU-test","host_index":0,"network":"none","nvidia_smi_pid":4242,"nvidia_smi_uuid":"GPU-test","outcome":"passed","requested_ordinal":0,"schema_version":"fathomdb.cuda-package-gpu-smoke/v1","smoke_pid":4242,"source_imported":false}
EOF
  done
  python3 - "$root" "$CANDIDATE" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
candidate = sys.argv[2]
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
manifest = {
    "schema_version": "fathomdb.cuda-package-rehearsal/v1",
    "candidate_sha": candidate,
    "route_receipt_sha256": digest(root / "route-receipt.json"),
    "preflight_witness_sha256": digest(root / "preflight-witness" / "cuda-preflight-witness.json"),
    "build_input": json.loads((root / "build-input.json").read_text()),
    "packages": {
        name: digest(root / "packages" / name)
        for name in (
            "fathomdb-0.8.23-cp311-cp311-manylinux_2_28_x86_64.whl",
            "fathomdb-0.8.23.tgz",
            "fathomdb-linux-x64-gnu-0.8.23.tgz",
        )
    },
    "smoke_evidence_sha256": {
        name: digest(root / "smoke" / name)
        for name in ("cpu-python.json", "cpu-napi.json", "gpu-python.json", "gpu-napi.json")
    },
}
(root / "cuda-package-rehearsal.json").write_text(
    json.dumps(manifest, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n"
)
PY
}

expect_accept() {
  local root="$1" description="$2"
  if python3 "$VERIFIER" --rehearsal-dir "$root" --candidate-sha "$CANDIDATE" >/dev/null; then
    pass "$description"
  else
    fail "$description"
  fi
}

expect_reject() {
  local root="$1" description="$2"
  if python3 "$VERIFIER" --rehearsal-dir "$root" --candidate-sha "$CANDIDATE" >/dev/null 2>&1; then
    fail "$description"
  fi
  pass "$description"
}

[ -f "$SCHEMA" ] || fail 'versioned package rehearsal schema exists'
[ -f "$VERIFIER" ] || fail 'fail-closed package rehearsal verifier exists'
[ -x "$REHEARSE" ] || fail 'package rehearsal helper exists'
pass 'package rehearsal control-plane files exist'

VALID="$TMPROOT/valid"
make_valid_bundle "$VALID"
expect_accept "$VALID" 'exact complete package rehearsal bundle is accepted'

cp -a "$VALID" "$TMPROOT/substituted"
printf 'substituted wheel\n' >> "$TMPROOT/substituted/packages/fathomdb-0.8.23-cp311-cp311-manylinux_2_28_x86_64.whl"
expect_reject "$TMPROOT/substituted" 'package byte substitution is rejected'

cp -a "$VALID" "$TMPROOT/unknown"
python3 - "$TMPROOT/unknown/cuda-package-rehearsal.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text())
value["unexpected"] = True
path.write_text(json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
expect_reject "$TMPROOT/unknown" 'unknown manifest field is rejected'

cp -a "$VALID" "$TMPROOT/extra"
printf 'extra\n' > "$TMPROOT/extra/packages/extra.tgz"
expect_reject "$TMPROOT/extra" 'extra package inventory member is rejected'

cp -a "$VALID" "$TMPROOT/cross-candidate"
python3 - "$TMPROOT/cross-candidate/cuda-package-rehearsal.json" "$OTHER_CANDIDATE" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text())
value["candidate_sha"] = sys.argv[2]
path.write_text(json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
expect_reject "$TMPROOT/cross-candidate" 'cross-candidate manifest is rejected'

cp -a "$VALID" "$TMPROOT/preflight-substituted"
printf '\n' >> "$TMPROOT/preflight-substituted/preflight-witness/cuda-preflight-witness.json"
expect_reject "$TMPROOT/preflight-substituted" 'preflight witness byte substitution is rejected'

cp -a "$VALID" "$TMPROOT/symlink"
mv "$TMPROOT/symlink/packages/fathomdb-0.8.23.tgz" "$TMPROOT/symlink/packages/real.tgz"
ln -s real.tgz "$TMPROOT/symlink/packages/fathomdb-0.8.23.tgz"
expect_reject "$TMPROOT/symlink" 'symlinked package is rejected'

cp -a "$VALID" "$TMPROOT/cpu-network"
python3 - "$TMPROOT/cpu-network/smoke/cpu-python.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text())
value["network"] = "host"
path.write_text(json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
expect_reject "$TMPROOT/cpu-network" 'CPU smoke with network access is rejected'

cp -a "$VALID" "$TMPROOT/gpu-correlation"
python3 - "$TMPROOT/gpu-correlation/smoke/gpu-napi.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text())
value["nvidia_smi_uuid"] = "GPU-other"
path.write_text(json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
expect_reject "$TMPROOT/gpu-correlation" 'GPU smoke without UUID correlation is rejected'

if "$REHEARSE" --candidate-sha "$CANDIDATE" --output-dir "$TMPROOT/no-inputs" >/dev/null 2>&1; then
  fail 'helper must not create a rehearsal without Slice 10 receipt and witness'
fi
pass 'helper requires Slice 10 receipt and witness before output'

printf '\nCUDA package rehearsal tests passed\n'
