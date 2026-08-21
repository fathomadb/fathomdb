#!/usr/bin/env bash
# Fail-closed tests for the retained CUDA package-rehearsal bundle.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERIFIER="$REPO_ROOT/scripts/release/verify-cuda-package-rehearsal.py"
SCHEMA="$REPO_ROOT/scripts/release/cuda-package-rehearsal.schema.json"
REHEARSE="$REPO_ROOT/scripts/release/cuda-package-rehearsal.sh"
SEAL_CLI="$REPO_ROOT/scripts/release/seal-cuda-cli-archive.sh"
CANDIDATE="0123456789abcdef0123456789abcdef01234567"
OTHER_CANDIDATE="89abcdef0123456789abcdef0123456789abcdef"
VERSION="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["workspace"]["package"]["version"])' "$REPO_ROOT/Cargo.toml")"
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
  version="$VERSION"
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
  local model_manifest_sha
  model_manifest_sha="$(sha256sum "$root/preflight-witness/model-cache-manifest.json")"
  model_manifest_sha="${model_manifest_sha%% *}"
  cat > "$root/build-input.json" <<EOF
{"archive_filename":"fathomdb-$version-x86_64-unknown-linux-gnu.tar.gz","candidate_sha":"$CANDIDATE","cli_features":["embed-cuda"],"model_cache_manifest_sha256":"$model_manifest_sha","napi_features":["default-embedder","embed-cuda"],"python_features":["embed-cuda","pyo3/extension-module"],"rerank_cuda":false,"schema_version":"fathomdb.cuda-package-build-input/v2","target":"x86_64-unknown-linux-gnu","version":"$version"}
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
      policy=auto status=cuda_unavailable effective='"cpu"' reason='"no_visible_cuda_device"' exit_code=0 ordinal_value=null environment='{}'
    else
      policy=cuda:0 status=cuda_unavailable effective=null reason='"no_visible_cuda_device"' exit_code=65 ordinal_value=0 environment='{"FATHOMDB_EMBED_DEVICE":"cuda:0"}'
    fi
    stdout="$root/smoke/$mode-cli-stdout.json"
    printf '{"schema_version":"fathomdb.doctor.gpu.v1","policy":"%s","cuda_compiled":true,"status":"%s","effective_device":%s,"devices":[],"reason":%s,"selected_uuid":null}\n' "$policy" "$status" "$effective" "$reason" > "$stdout"
    digest="$(sha256sum "$stdout" | cut -d' ' -f1)"
    cat > "$root/smoke/$mode-cli.json" <<EOF
{"archive_filename":"fathomdb-$version-x86_64-unknown-linux-gnu.tar.gz","archive_sha256":"PLACEHOLDER","argv":["/tmp/fathomdb-cli/fathomdb-$version-x86_64-unknown-linux-gnu/fathomdb","doctor","gpu","--json"],"consumer":"cli","doctor_output_filename":"$mode-cli-stdout.json","doctor_output_sha256":"$digest","effective_device":$effective,"environment":$environment,"evidence_provenance":"installed_candidate","exit_code":$exit_code,"isolation":{"database_opened":false,"model_loaded":false,"network":"none","source_checkout_mounted":false},"reason":$reason,"requested_ordinal":$ordinal_value,"requested_policy":"$policy","schema_version":"fathomdb.cuda-package-cli-smoke/v2","status":"$status","target":"x86_64-unknown-linux-gnu"}
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
    "pending_external": ["compatible_gpu_cli", "incompatible_classifier_observation"],
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
[ -f "$REPO_ROOT/scripts/release/cuda-rerank-package-rehearsal.schema.json" ] || fail 'v3 reranker package rehearsal schema exists'
[ -f "$VERIFIER" ] || fail 'fail-closed package rehearsal verifier exists'
[ -x "$REHEARSE" ] || fail 'package rehearsal helper exists'
[ -x "$SEAL_CLI" ] || fail 'deterministic CLI archive sealer exists'
pass 'package rehearsal control-plane files exist'

printf '#!/bin/sh\nexit 0\n' > "$TMPROOT/fathomdb"
chmod 0755 "$TMPROOT/fathomdb"
"$SEAL_CLI" --binary "$TMPROOT/fathomdb" --version "$VERSION" --output "$TMPROOT/cli-a.tar.gz"
"$SEAL_CLI" --binary "$TMPROOT/fathomdb" --version "$VERSION" --output "$TMPROOT/cli-b.tar.gz"
cmp -s "$TMPROOT/cli-a.tar.gz" "$TMPROOT/cli-b.tar.gz" || fail 'CLI archive sealer is not byte deterministic'
pass 'CLI archive sealer is byte deterministic'

VALID="$TMPROOT/valid"
make_valid_bundle "$VALID"
expect_accept "$VALID" 'exact complete package rehearsal bundle is accepted'

cp -a "$VALID" "$TMPROOT/main-route-v2"
python3 - "$TMPROOT/main-route-v2" "$CANDIDATE" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

root, candidate = map(Path, sys.argv[1:])
route = {
    "candidate_sha": str(candidate),
    "run_attempt": 1,
    "run_id": 1,
    "schema_version": "fathomdb.cuda-main-route-receipt/v2",
    "workflow_ref": "fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main",
    "workflow_sha": "a" * 40,
}
route_path = root / "route-receipt.json"
route_path.write_text(json.dumps(route, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
manifest_path = root / "cuda-package-rehearsal.json"
manifest = json.loads(manifest_path.read_text())
manifest["route_receipt_sha256"] = hashlib.sha256(route_path.read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
expect_accept "$TMPROOT/main-route-v2" 'main-route v2 receipt is accepted for a dry run'

cp -a "$VALID" "$TMPROOT/rerank-v3"
python3 - "$TMPROOT/rerank-v3" "$REPO_ROOT" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

root, repo = map(Path, sys.argv[1:])
build_path = root / "build-input.json"
build = json.loads(build_path.read_text())
build.update({
    "schema_version": "fathomdb.cuda-package-build-input/v3",
    "python_features": ["embed-cuda", "rerank-cuda", "pyo3/extension-module"],
    "napi_features": ["default-embedder", "embed-cuda", "rerank-cuda"],
    "cli_features": ["embed-cuda", "rerank-cuda"],
    "rerank_cuda": True,
})
build_path.write_text(json.dumps(build, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
preflight_build_path = root / "preflight-witness" / "build-input.json"
preflight_build = json.loads(preflight_build_path.read_text())
preflight_build.update({
    "schema_version": "fathomdb.cuda-preflight-build-input/v3",
    "python_features": ["embed-cuda", "rerank-cuda", "pyo3/extension-module"],
    "napi_features": ["default-embedder", "embed-cuda", "rerank-cuda"],
    "rerank_cuda": True,
})
preflight_build_path.write_text(json.dumps(preflight_build, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
preflight_witness_path = root / "preflight-witness" / "cuda-preflight-witness.json"
preflight_witness = json.loads(preflight_witness_path.read_text())
preflight_witness["schema_version"] = "fathomdb.cuda-preflight-witness/v3"
tokenizer_digest = "d241a60d5e8f04cc" + "1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66"
reranker_manifest = {"schema_version":"fathomdb.reranker-cache/v1","repository":"cross-encoder/ms-marco-TinyBERT-L2-v2","revision":"81d1926f67cb8eee2c2be17ca9f793c7c3bd20cc","snapshot_relpath":"fathomdb/reranker/0290849b0459","files":{"config.json":"2144195e107cd7ea61556478e7add12986ebfbc3085f924fc0b90c2410604879","tokenizer.json":tokenizer_digest,"model.safetensors":"a0e7364ddf91ff7028f1102e1b91ac7a72e3db4061241bd84efe45c72c9af03a"}}
preflight_root = root / "preflight-witness"
(preflight_root / "reranker-cache-manifest.json").write_text(json.dumps(reranker_manifest, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
reranker_digest = hashlib.sha256((preflight_root / "reranker-cache-manifest.json").read_bytes()).hexdigest()
for consumer in ("python", "napi"):
    (preflight_root / f"reranker-{consumer}-cpu-smoke.json").write_text(json.dumps({"schema_version":"fathomdb.cuda-reranker-cpu-smoke/v1","consumer":consumer,"requested_policy":"auto","effective_device":"cpu","reason":"no_visible_cuda_device","network":"none","source_imported":False,"rerank_performed":True,"reranker_cache_manifest_sha256":reranker_digest,"reranker_cache_read_only":True,"reranker_device_environment":"unset"}, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
    message = "cuda:0 requested for reranking but unavailable: NoVisibleCudaDevice"
    stdout = preflight_root / f"forced-reranker-{consumer}-stdout.txt"
    stderr = preflight_root / f"forced-reranker-{consumer}-stderr.txt"
    argv = ["/opt/python/cp311-cp311/bin/python", "/fathomdb-harness/forced-reranker-python.py"] if consumer == "python" else ["node", "/fathomdb-harness/forced-reranker-napi.mjs"]
    stdout.write_text(json.dumps({"schema_version":"fathomdb.cuda-forced-reranker-capture/v1","consumer":consumer,"argv":argv,"requested_policy":"cuda:0","status":"cuda_unavailable","effective_device":None,"reason":"no_visible_cuda_device","error":{"type":"RerankerDevicePolicyError","kind":"no_visible_cuda_device","ordinal":0,"message":message}}, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
    stderr.write_text(message + "\n")
    (preflight_root / f"forced-reranker-{consumer}.json").write_text(json.dumps({"schema_version":"fathomdb.cuda-forced-reranker-failure/v1","consumer":consumer,"requested_policy":"cuda:0","cuda_compiled":True,"visible_devices":[],"status":"cuda_unavailable","effective_device":None,"reason":"no_visible_cuda_device","provenance":"installed_candidate","command":f"installed_{consumer}_engine_open_without_default_embedder","exit_code":1,"stdout_filename":stdout.name,"stdout_sha256":hashlib.sha256(stdout.read_bytes()).hexdigest(),"stderr_filename":stderr.name,"stderr_sha256":hashlib.sha256(stderr.read_bytes()).hexdigest()}, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
for name in ("forced-reranker-python.py", "forced-reranker-napi.mjs"):
    (preflight_root / name).write_bytes((repo / "scripts/release" / name).read_bytes())
preflight_build_digest = hashlib.sha256(preflight_build_path.read_bytes()).hexdigest()
preflight_witness["build_input_sha256"] = preflight_build_digest
preflight_witness["evidence_sha256"] = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in preflight_root.iterdir() if path.name != preflight_witness_path.name}
preflight_witness_path.write_text(json.dumps(preflight_witness, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
manifest_path = root / "cuda-package-rehearsal.json"
manifest = json.loads(manifest_path.read_text())
manifest["schema_version"] = "fathomdb.cuda-package-rehearsal/v3"
manifest["pending_external"] = ["compatible_gpu_reranker_cli", "incompatible_reranker_classifier_observation"]
manifest["build_input"] = build
manifest["preflight_witness_sha256"] = hashlib.sha256(preflight_witness_path.read_bytes()).hexdigest()
archive = manifest["packages"]["cli_archive"]
raw = json.dumps({"schema_version":"fathomdb.doctor.reranker-gpu.v1","subsystem":"reranker","policy":"auto","cuda_compiled":True,"effective_device":"cpu","devices":[],"reason":"no_visible_cuda_device","selected_uuid":None}, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"
smoke = root / "smoke"
(smoke / "reranker-cli-doctor-stdout.json").write_bytes(raw)
version = manifest["version"]
record = {"schema_version":"fathomdb.cuda-reranker-cli-doctor/v1","consumer":"cli","archive_filename":archive["filename"],"archive_sha256":archive["sha256"],"target":"x86_64-unknown-linux-gnu","argv":[f"/tmp/fathomdb-cli/fathomdb-{version}-x86_64-unknown-linux-gnu/fathomdb","doctor","reranker-gpu","--json"],"requested_policy":"auto","environment":{},"isolation":{"database_opened":False,"model_loaded":False,"network":"none","source_checkout_mounted":False},"evidence_provenance":"installed_candidate","exit_code":0,"doctor_output_filename":"reranker-cli-doctor-stdout.json","doctor_output_sha256":hashlib.sha256(raw).hexdigest(),"effective_device":"cpu","reason":"no_visible_cuda_device"}
(smoke / "reranker-cli-doctor.json").write_text(json.dumps(record, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
manifest["smoke_evidence_sha256"] = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in smoke.iterdir()}
manifest_path.write_text(json.dumps(manifest, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
expect_accept "$TMPROOT/rerank-v3" 'v3 reranker feature tuple is accepted with GPU receipts PENDING_EXTERNAL'

python3 - "$TMPROOT/future-reranker-gpu-receipt.json" "$CANDIDATE" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
candidate = sys.argv[2]
path.write_text(json.dumps({
    "schema_version": "fathomdb.cuda-reranker-gpu-inference-receipt/v1",
    "candidate_sha": candidate, "consumer": "cli", "target": "x86_64-unknown-linux-gnu",
    "requested_policy": "cuda:0", "status": "selected_cuda", "effective_device": "cuda:0",
    "visible_devices": [{"visible_ordinal": 0, "uuid": "GPU-test", "name": "test GPU", "compute_capability": "8.0"}],
    "selected_uuid": "GPU-test", "nvidia_smi_uuid": "GPU-test", "process_id": 42,
    "nvidia_smi_compute_process_id": 42, "model_cache_manifest_sha256": "a" * 64,
    "rerank_performed": True, "network": "none", "source_imported": False,
}, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
python3 "$VERIFIER" --rehearsal-dir "$TMPROOT/rerank-v3" --candidate-sha "$CANDIDATE" \
  --future-reranker-gpu-receipt "$TMPROOT/future-reranker-gpu-receipt.json" >/dev/null
pass 'future reranker GPU receipt schema is independently verifiable without promotion'

cp -a "$TMPROOT/rerank-v3" "$TMPROOT/rerank-v3-missing-feature"
python3 - "$TMPROOT/rerank-v3-missing-feature/build-input.json" "$TMPROOT/rerank-v3-missing-feature/cuda-package-rehearsal.json" <<'PY'
import json
from pathlib import Path
import sys
build_path, manifest_path = map(Path, sys.argv[1:])
build = json.loads(build_path.read_text()); build["cli_features"] = ["embed-cuda"]
build_path.write_text(json.dumps(build, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
manifest = json.loads(manifest_path.read_text()); manifest["build_input"] = build
manifest_path.write_text(json.dumps(manifest, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
expect_reject "$TMPROOT/rerank-v3-missing-feature" 'v3 reranker route rejects a missing CLI rerank-cuda feature'

cp -a "$VALID" "$TMPROOT/v1"
sed -i 's#fathomdb.cuda-package-rehearsal/v2#fathomdb.cuda-package-rehearsal/v1#' "$TMPROOT/v1/cuda-package-rehearsal.json"
expect_reject "$TMPROOT/v1" 'legacy v1 rehearsal is rejected'

cp -a "$VALID" "$TMPROOT/version-mismatch"
python3 - "$TMPROOT/version-mismatch/cuda-package-rehearsal.json" <<'PY'
import json, sys
from pathlib import Path
path = Path(sys.argv[1]); value = json.loads(path.read_text())
value["packages"]["npm_main"]["version"] = "9.9.9"
path.write_text(json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
expect_reject "$TMPROOT/version-mismatch" 'cross-version package coordinate is rejected'

cp -a "$VALID" "$TMPROOT/cli-output-substitution"
printf ' ' >> "$TMPROOT/cli-output-substitution/smoke/cpu-cli-stdout.json"
expect_reject "$TMPROOT/cli-output-substitution" 'raw CLI doctor output substitution is rejected'

cp -a "$VALID" "$TMPROOT/cli-archive-substitution"
printf 'x' >> "$TMPROOT/cli-archive-substitution/packages/fathomdb-$VERSION-x86_64-unknown-linux-gnu.tar.gz"
expect_reject "$TMPROOT/cli-archive-substitution" 'CLI archive byte substitution is rejected'

cp -a "$VALID" "$TMPROOT/substituted"
printf 'substituted wheel\n' >> "$TMPROOT/substituted/packages/fathomdb-$VERSION-cp311-cp311-manylinux_2_28_x86_64.whl"
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
mv "$TMPROOT/symlink/packages/fathomdb-$VERSION.tgz" "$TMPROOT/symlink/packages/real.tgz"
ln -s real.tgz "$TMPROOT/symlink/packages/fathomdb-$VERSION.tgz"
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
