#!/usr/bin/env bash
# Regression coverage for the versioned, SHA-bound CUDA preflight witness.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PREFLIGHT="$REPO_ROOT/scripts/release/cuda-preflight.sh"
VERIFIER="$REPO_ROOT/scripts/release/verify-cuda-preflight-witness.py"
SCHEMA="$REPO_ROOT/scripts/release/cuda-preflight-witness.schema.json"
CANDIDATE="0123456789abcdef0123456789abcdef01234567"
OTHER_CANDIDATE="89abcdef0123456789abcdef0123456789abcdef"
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

fail() {
  printf 'FAIL  %s\n' "$1" >&2
  exit 1
}

pass() {
  printf 'PASS  %s\n' "$1"
}

require_source_contract() {
  local scrub count
  scrub='env -u FATHOMDB_EMBED_DEVICE -u FATHOMDB_RERANK_DEVICE -u CUDA_VISIBLE_DEVICES -u NVIDIA_VISIBLE_DEVICES -u HIP_VISIBLE_DEVICES -u ROCR_VISIBLE_DEVICES'

  [ -f "$PREFLIGHT" ] || fail "CUDA preflight exists"
  count="$(grep -Fc -- "$scrub" "$PREFLIGHT" || true)"
  [ "$count" -eq 2 ] || fail "both driverless smokes scrub device-selection variables"
  if grep -Fq -- '-e FATHOMDB_EMBED_DEVICE=cpu' "$PREFLIGHT"; then
    fail "driverless smokes do not force an embedder device"
  fi
  if grep -Fq -- '-e FATHOMDB_RERANK_DEVICE=' "$PREFLIGHT"; then
    fail "driverless smokes do not force a reranker device"
  fi
  pass "driverless smokes prove CPU-default behavior without device selection"
}

write_valid_witness() {
  local directory="$1"
  mkdir -p "$directory"
  python3 - "$directory" "$CANDIDATE" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

directory = Path(sys.argv[1])
candidate = sys.argv[2]
names = (
    "environment.txt",
    "manylinux-build.txt",
    "dynamic-dependencies.txt",
    "python-auditwheel.txt",
    "driverless-python-cpu-smoke.txt",
    "driverless-napi-cpu-smoke.txt",
    "gpu-python-cuda-witness.txt",
    "gpu-node-cuda-witness.txt",
    "gpu-node-cuda-smoke.txt",
)
evidence = {}
for name in names:
    path = directory / name
    path.write_text(f"verified CUDA preflight evidence: {name}\n", encoding="utf-8")
    evidence[name] = hashlib.sha256(path.read_bytes()).hexdigest()
(directory / "cuda-preflight-witness.json").write_text(
    json.dumps(
        {
            "schema_version": "fathomdb.cuda-preflight-witness/v1",
            "candidate_sha": candidate,
            "outcome": "passed",
            "evidence_sha256": evidence,
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
}

expect_reject() {
  local directory="$1" candidate="$2" description="$3"
  if python3 "$VERIFIER" --witness-dir "$directory" --candidate-sha "$candidate" >/dev/null 2>&1; then
    fail "$description"
  fi
  pass "$description"
}

require_source_contract
[ -f "$SCHEMA" ] || fail "versioned preflight witness schema exists"
[ -f "$VERIFIER" ] || fail "fail-closed preflight witness verifier exists"

VALID="$TMPROOT/valid"
write_valid_witness "$VALID"
if ! python3 "$VERIFIER" --witness-dir "$VALID" --candidate-sha "$CANDIDATE" >/dev/null; then
  fail "complete witness for the requested candidate is accepted"
fi
pass "complete witness for the requested candidate is accepted"

expect_reject "$TMPROOT/missing" "$CANDIDATE" "missing witness is rejected"

MALFORMED="$TMPROOT/malformed"
write_valid_witness "$MALFORMED"
printf '{not-json}\n' > "$MALFORMED/cuda-preflight-witness.json"
expect_reject "$MALFORMED" "$CANDIDATE" "malformed witness is rejected"

expect_reject "$VALID" "$OTHER_CANDIDATE" "wrong candidate SHA is rejected"

INCOMPLETE="$TMPROOT/incomplete"
write_valid_witness "$INCOMPLETE"
rm "$INCOMPLETE/gpu-node-cuda-witness.txt"
expect_reject "$INCOMPLETE" "$CANDIDATE" "incomplete witness is rejected"

TAMPERED="$TMPROOT/tampered"
write_valid_witness "$TAMPERED"
printf 'tampered\n' >> "$TAMPERED/environment.txt"
expect_reject "$TAMPERED" "$CANDIDATE" "evidence digest mismatch is rejected"

EMPTY="$TMPROOT/empty"
write_valid_witness "$EMPTY"
: > "$EMPTY/manylinux-build.txt"
python3 - "$EMPTY" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

directory = Path(sys.argv[1])
witness_path = directory / "cuda-preflight-witness.json"
witness = json.loads(witness_path.read_text(encoding="utf-8"))
witness["evidence_sha256"]["manylinux-build.txt"] = hashlib.sha256(
    (directory / "manylinux-build.txt").read_bytes()
).hexdigest()
witness_path.write_text(
    json.dumps(witness, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
PY
expect_reject "$EMPTY" "$CANDIDATE" "empty evidence is rejected even with a matching digest"

ROOT_LINK="$TMPROOT/root-link"
write_valid_witness "$ROOT_LINK"
mv "$ROOT_LINK/cuda-preflight-witness.json" "$ROOT_LINK/real-witness.json"
ln -s real-witness.json "$ROOT_LINK/cuda-preflight-witness.json"
expect_reject "$ROOT_LINK" "$CANDIDATE" "symlinked root witness is rejected"

EVIDENCE_LINK="$TMPROOT/evidence-link"
write_valid_witness "$EVIDENCE_LINK"
mv "$EVIDENCE_LINK/environment.txt" "$EVIDENCE_LINK/real-environment.txt"
ln -s real-environment.txt "$EVIDENCE_LINK/environment.txt"
expect_reject "$EVIDENCE_LINK" "$CANDIDATE" "symlinked witness evidence is rejected"

printf '\nCUDA preflight witness tests passed\n'
