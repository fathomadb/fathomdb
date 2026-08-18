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
  local version
  version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$REPO_ROOT/Cargo.toml" | head -1)"
  printf 'wheel bytes\n' > "$root/packages/fathomdb-$version-cp311-cp311-manylinux_2_28_x86_64.whl"
  printf 'thin npm bytes\n' > "$root/packages/fathomdb-$version.tgz"
  printf 'napi bytes\n' > "$root/packages/fathomdb-linux-x64-gnu-$version.tgz"
  mkdir -p "$root/archive/fathomdb-$version-x86_64-unknown-linux-gnu"
  printf '#!/bin/sh\nexit 0\n' > "$root/archive/fathomdb-$version-x86_64-unknown-linux-gnu/fathomdb"
  chmod 0755 "$root/archive" "$root/archive/fathomdb-$version-x86_64-unknown-linux-gnu" \
    "$root/archive/fathomdb-$version-x86_64-unknown-linux-gnu/fathomdb"
  tar --format=posix --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
    --pax-option=delete=atime,delete=ctime --mode='u=rwx,go=rx' \
    -C "$root/archive" -cf - "fathomdb-$version-x86_64-unknown-linux-gnu" \
    | gzip -n > "$root/packages/fathomdb-$version-x86_64-unknown-linux-gnu.tar.gz"
  rm -rf "$root/archive"
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
  cat > "$root/build-input.json" <<EOF
{"archive_filename":"fathomdb-$version-x86_64-unknown-linux-gnu.tar.gz","candidate_sha":"$CANDIDATE","cli_features":["embed-cuda"],"model_cache_manifest_sha256":"$(sha256sum "$root/preflight-witness/model-cache-manifest.json" | cut -d' ' -f1)","napi_features":["default-embedder","embed-cuda"],"python_features":["embed-cuda","pyo3/extension-module"],"rerank_cuda":false,"schema_version":"fathomdb.cuda-package-build-input/v2","target":"x86_64-unknown-linux-gnu","version":"$version"}
EOF
  for consumer in python napi; do
    cat > "$root/smoke/cpu-$consumer.json" <<EOF
{"consumer":"$consumer","environment":"env -i","gpu_nodes_visible":false,"network":"none","outcome":"passed","schema_version":"fathomdb.cuda-package-cpu-smoke/v1","source_imported":false}
EOF
    cat > "$root/smoke/gpu-$consumer.json" <<EOF
{"consumer":"$consumer","device_name":"NVIDIA test","driver_version":"999.99","gpu_uuid":"GPU-test","host_index":0,"network":"none","nvidia_smi_pid":4242,"nvidia_smi_uuid":"GPU-test","outcome":"passed","requested_ordinal":0,"schema_version":"fathomdb.cuda-package-gpu-smoke/v1","smoke_pid":4242,"source_imported":false}
EOF
  done
  for mode in cpu forced-cuda-unavailable; do
    if [ "$mode" = cpu ]; then
      policy=auto status=selected_cpu_no_cuda effective='"cpu"' reason='"cuda_not_available"' exit_code=0
    else
      policy=cuda:0 status=cuda_unavailable effective=null reason='"no_visible_cuda_device"' exit_code=65
    fi
    stdout="$root/smoke/$mode-cli-stdout.json"
    printf '{"cuda_compiled":true,"devices":[],"effective_device":%s,"policy":"%s","reason":%s,"schema_version":"fathomdb.doctor.gpu/v1","selected_uuid":null,"status":"%s"}\n' "$effective" "$policy" "$reason" "$status" > "$stdout"
    digest="$(sha256sum "$stdout" | cut -d' ' -f1)"
    cat > "$root/smoke/$mode-cli.json" <<EOF
{"archive_filename":"fathomdb-$version-x86_64-unknown-linux-gnu.tar.gz","archive_sha256":"PLACEHOLDER","argv":["/fathomdb-cli/fathomdb-$version-x86_64-unknown-linux-gnu/fathomdb","doctor","gpu","--json"],"consumer":"cli","doctor_output_filename":"$mode-cli-stdout.json","doctor_output_sha256":"$digest","effective_device":$effective,"environment":{"FATHOMDB_EMBED_DEVICE":"$policy"},"evidence_provenance":"installed_candidate","exit_code":$exit_code,"isolation":{"database_opened":false,"model_loaded":false,"network":"none","source_checkout_mounted":false},"reason":$reason,"requested_ordinal":$( [ "$mode" = cpu ] && printf null || printf 0 ),"requested_policy":"$policy","schema_version":"fathomdb.cuda-package-cli-smoke/v2","status":"$status","target":"x86_64-unknown-linux-gnu"}
EOF
  done
  python3 - "$root" "$CANDIDATE" "$REPO_ROOT" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
candidate = sys.argv[2]
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
version = (Path(sys.argv[3]) / "Cargo.toml").read_text().split('version = "', 1)[1].split('"', 1)[0]
packages = {}
for kind, name, contains_cuda in (
    ("python_wheel", f"fathomdb-{version}-cp311-cp311-manylinux_2_28_x86_64.whl", True),
    ("npm_main", f"fathomdb-{version}.tgz", False),
    ("napi_platform", f"fathomdb-linux-x64-gnu-{version}.tgz", True),
    ("cli_archive", f"fathomdb-{version}-x86_64-unknown-linux-gnu.tar.gz", True),
):
    packages[kind] = {"contains_cuda": contains_cuda, "filename": name, "sha256": digest(root / "packages" / name), "target": "x86_64-unknown-linux-gnu", "version": version}
cli_sha = packages["cli_archive"]["sha256"]
for path in (root / "smoke").glob("*-cli.json"):
    value = json.loads(path.read_text())
    value["archive_sha256"] = cli_sha
    path.write_text(json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
manifest = {
    "schema_version": "fathomdb.cuda-package-rehearsal/v2",
    "candidate_sha": candidate,
    "version": version,
    "target": "x86_64-unknown-linux-gnu",
    "route_receipt_sha256": digest(root / "route-receipt.json"),
    "preflight_witness_sha256": digest(root / "preflight-witness" / "cuda-preflight-witness.json"),
    "build_input": json.loads((root / "build-input.json").read_text()),
    "packages": packages,
    "smoke_evidence_sha256": {
        name: digest(root / "smoke" / name)
        for name in ("cpu-python.json", "cpu-napi.json", "gpu-python.json", "gpu-napi.json", "cpu-cli.json", "cpu-cli-stdout.json", "forced-cuda-unavailable-cli.json", "forced-cuda-unavailable-cli-stdout.json")
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
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$REPO_ROOT/Cargo.toml" | head -1)"
printf 'substituted wheel\n' >> "$TMPROOT/substituted/packages/fathomdb-$version-cp311-cp311-manylinux_2_28_x86_64.whl"
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
mv "$TMPROOT/symlink/packages/fathomdb-$version.tgz" "$TMPROOT/symlink/packages/real.tgz"
ln -s real.tgz "$TMPROOT/symlink/packages/fathomdb-$version.tgz"
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
