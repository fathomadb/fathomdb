#!/usr/bin/env bash
# Contract test for R32-07 tightened failure diagnostics across:
#   - scripts/check-sdk-surface-parity.py
#   - scripts/check-governed-surface-pin.sh
#   - dev/tools/module-boundary-gate/src/{main,config}.rs
#
# Every failure arm uses throwaway fixtures under mktemp -d; no real repository
# contract, policy, or source file is mutated.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PARITY_CHECKER="$REPO_ROOT/scripts/check-sdk-surface-parity.py"
PIN_CHECKER="$REPO_ROOT/scripts/check-governed-surface-pin.sh"
ALLOWLIST="$REPO_ROOT/src/conformance/governed-surface-allowlist.json"
PARITY_MAP="$REPO_ROOT/src/conformance/governed-operation-parity.json"
SURFACE_PIN="$REPO_ROOT/scripts/governed-surface-pin.json"
TOOL_DIR="$REPO_ROOT/dev/tools/module-boundary-gate"
GATE_BIN="$TOOL_DIR/target/debug/fathomdb-module-boundary-gate"

# Select a Python >= 3.11 interpreter (check-sdk-surface-parity.py uses tomllib).
PY_BIN=""
for candidate in "$REPO_ROOT/.venv/bin/python" python3.13 python3.12 python3.11 python3; do
  if { [ -x "$candidate" ] || command -v "$candidate" >/dev/null 2>&1; } &&
    "$candidate" -c 'import sys; raise SystemExit(sys.version_info < (3, 11))'; then
    PY_BIN="$candidate"
    break
  fi
done
if [ -z "$PY_BIN" ]; then
  printf 'FAIL  test_gate_diagnostics_0832 requires Python >= 3.11\n' >&2
  exit 1
fi

FAILED=0
pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

require_contains() {
  local haystack="$1" needle="$2" label="$3"
  if grep -Fq -- "$needle" <<<"$haystack"; then
    pass "$label"
  else
    fail "$label (missing '$needle') -- output was:\n$haystack"
  fi
}

TMPROOT="$(mktemp -d)"
cleanup() {
  case "$TMPROOT" in
    "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$TMPROOT" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$TMPROOT" >&2 ;;
  esac
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# 1. scripts/check-sdk-surface-parity.py failure diagnostics
# ---------------------------------------------------------------------------
DRIFT_PARITY="$TMPROOT/governed-operation-parity.json"
"$PY_BIN" - "$PARITY_MAP" "$DRIFT_PARITY" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as fh:
    data = json.load(fh)
data["operations"][0]["signed_members"].append("synthetic.drift_member")
with open(sys.argv[2], "w", encoding="utf-8") as fh:
    json.dump(data, fh, indent=2)
    fh.write("\n")
PY

set +e
PARITY_CONTRACT_OUT="$("$PY_BIN" "$PARITY_CHECKER" --signed "$ALLOWLIST" --companion "$DRIFT_PARITY" 2>&1)"
PARITY_CONTRACT_RC=$?
set -e
if [ "$PARITY_CONTRACT_RC" -ne 0 ]; then
  pass "parity contract drift fails non-zero"
else
  fail "parity contract drift unexpectedly exited 0"
fi
require_contains "$PARITY_CONTRACT_OUT" "src/conformance/governed-operation-parity.json" \
  "parity contract failure names governed-operation-parity.json"
require_contains "$PARITY_CONTRACT_OUT" "src/conformance/governed-surface-allowlist.json" \
  "parity contract failure names governed-surface-allowlist.json"
require_contains "$PARITY_CONTRACT_OUT" "scripts/governed-surface-pin.json" \
  "parity contract failure names governed-surface-pin.json"
require_contains "$PARITY_CONTRACT_OUT" "scripts/check-sdk-surface-parity.py" \
  "parity contract failure names check-sdk-surface-parity.py"
require_contains "$PARITY_CONTRACT_OUT" "src/rust/crates/fathomdb-sdk/src/" \
  "parity contract failure names Rust SDK binding source path"
require_contains "$PARITY_CONTRACT_OUT" "src/python/fathomdb/" \
  "parity contract failure names Python binding source path"
require_contains "$PARITY_CONTRACT_OUT" "src/ts/src/" \
  "parity contract failure names TypeScript binding source path"

for pair in \
  "rust|src/rust/crates/fathomdb-sdk/src/" \
  "python|src/python/fathomdb/" \
  "typescript|src/ts/src/"; do
  binding="${pair%%|*}"
  expected_path="${pair#*|}"
  set +e
  OBS_OUT="$("$PY_BIN" "$PARITY_CHECKER" --signed "$ALLOWLIST" --companion "$PARITY_MAP" --binding "$binding" --observed-json '[]' 2>&1)"
  OBS_RC=$?
  set -e
  if [ "$OBS_RC" -ne 0 ]; then
    pass "parity observed drift for $binding fails non-zero"
  else
    fail "parity observed drift for $binding unexpectedly exited 0"
  fi
  require_contains "$OBS_OUT" "src/conformance/governed-operation-parity.json" \
    "parity $binding failure names governed-operation-parity.json"
  require_contains "$OBS_OUT" "src/conformance/governed-surface-allowlist.json" \
    "parity $binding failure names governed-surface-allowlist.json"
  require_contains "$OBS_OUT" "scripts/governed-surface-pin.json" \
    "parity $binding failure names governed-surface-pin.json"
  require_contains "$OBS_OUT" "$expected_path" \
    "parity $binding failure names binding source path $expected_path"
done

# ---------------------------------------------------------------------------
# 2. scripts/check-governed-surface-pin.sh failure diagnostics
# ---------------------------------------------------------------------------
DRIFT_ALLOWLIST="$TMPROOT/governed-surface-allowlist.json"
"$PY_BIN" - "$ALLOWLIST" "$DRIFT_ALLOWLIST" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as fh:
    data = json.load(fh)
data["allowlist"].append("synthetic.unsigned_op")
with open(sys.argv[2], "w", encoding="utf-8") as fh:
    json.dump(data, fh, indent=2)
    fh.write("\n")
PY

set +e
PIN_DRIFT_OUT="$(bash "$PIN_CHECKER" --file "$DRIFT_ALLOWLIST" --pin "$SURFACE_PIN" 2>&1)"
PIN_DRIFT_RC=$?
set -e
if [ "$PIN_DRIFT_RC" -eq 1 ]; then
  pass "governed-surface-pin divergence exits 1"
else
  fail "governed-surface-pin divergence expected exit 1, got $PIN_DRIFT_RC"
fi
require_contains "$PIN_DRIFT_OUT" "governed-operation-parity.json" \
  "governed-surface-pin divergence cross-references governed-operation-parity.json"
require_contains "$PIN_DRIFT_OUT" "check-sdk-surface-parity.py" \
  "governed-surface-pin divergence cross-references check-sdk-surface-parity.py"
require_contains "$PIN_DRIFT_OUT" "dev/design/cross-cutting-change-checklist.md" \
  "governed-surface-pin divergence cross-references cross-cutting-change-checklist.md"
require_contains "$PIN_DRIFT_OUT" "DO NOT update the pin to make this pass" \
  "governed-surface-pin divergence preserves HITL routing warning"

MALFORMED_PIN="$TMPROOT/malformed-pin.json"
printf '{"sha256": ""}\n' >"$MALFORMED_PIN"
set +e
PIN_ENV_OUT="$(bash "$PIN_CHECKER" --file "$ALLOWLIST" --pin "$MALFORMED_PIN" 2>&1)"
PIN_ENV_RC=$?
set -e
if [ "$PIN_ENV_RC" -eq 2 ]; then
  pass "governed-surface-pin malformed pin exits 2"
else
  fail "governed-surface-pin malformed pin expected exit 2, got $PIN_ENV_RC"
fi
require_contains "$PIN_ENV_OUT" "governed-operation-parity.json" \
  "governed-surface-pin env error cross-references governed-operation-parity.json"
require_contains "$PIN_ENV_OUT" "check-sdk-surface-parity.py" \
  "governed-surface-pin env error cross-references check-sdk-surface-parity.py"
require_contains "$PIN_ENV_OUT" "dev/design/cross-cutting-change-checklist.md" \
  "governed-surface-pin env error cross-references cross-cutting-change-checklist.md"

# ---------------------------------------------------------------------------
# 3. dev/tools/module-boundary-gate/src/{main,config}.rs failure diagnostics
# ---------------------------------------------------------------------------
# Ensure the gate binary is built from the current worktree sources.
"$REPO_ROOT/scripts/check-module-boundaries.sh" >/dev/null

make_boundary_fixture() {
  local root="$1"
  mkdir -p "$root/dev/tools" "$root/src/rust/crates/fathomdb-engine/src"
  cat >"$root/src/rust/crates/fathomdb-engine/Cargo.toml" <<'TOML'
[package]
name = "fathomdb-engine"
version = "0.0.0"

[features]
default = []
test-hooks = []
TOML
  cat >"$root/src/rust/crates/fathomdb-engine/src/lib.rs" <<'RS'
mod reader_pool;
mod search;
mod search_api;
mod read;
mod graph_expand;
mod filter;
mod frozen_read;

pub struct Engine {
    pub(crate) reader_pool: u8,
}

impl Engine {
    pub fn open() {}
}
RS
  for m in reader_pool search search_api read graph_expand filter frozen_read; do
    printf 'pub(crate) fn stub_%s() {}\n' "$m" >"$root/src/rust/crates/fathomdb-engine/src/${m}.rs"
  done
  cat >"$root/dev/tools/module-boundary-policy.txt" <<'POLICY'
version 1
admitted root
governed reader_pool
governed search
governed search_api
governed read
governed graph_expand
governed filter
governed frozen_read
field reader_pool reader_pool
forbid-cycle graph_expand reader_pool
forbid-cycle graph_expand search
forbid-cycle graph_expand search_api
forbid-cycle read reader_pool
forbid-cycle reader_pool search
forbid-dependency filter search
forbid-dependency frozen_read search
forbid-dependency graph_expand reader_pool
forbid-dependency graph_expand search
forbid-dependency graph_expand search_api
forbid-dependency read reader_pool
forbid-dependency read search
forbid-dependency search reader_pool
POLICY
}

# 3a. Missing module classification
FIX_CLASS="$TMPROOT/boundary-missing-classification"
make_boundary_fixture "$FIX_CLASS"
grep -v '^governed filter$' "$FIX_CLASS/dev/tools/module-boundary-policy.txt" >"$FIX_CLASS/policy.tmp"
mv "$FIX_CLASS/policy.tmp" "$FIX_CLASS/dev/tools/module-boundary-policy.txt"
set +e
CLASS_OUT="$("$GATE_BIN" --root "$FIX_CLASS" 2>&1)"
CLASS_RC=$?
set -e
if [ "$CLASS_RC" -ne 0 ]; then
  pass "module-boundary missing classification fails non-zero"
else
  fail "module-boundary missing classification unexpectedly exited 0"
fi
require_contains "$CLASS_OUT" "module classification missing filter" \
  "module-boundary missing classification preserves existing diagnostic substring"
require_contains "$CLASS_OUT" "dev/tools/module-boundary-policy.txt" \
  "module-boundary missing classification names module-boundary-policy.txt"
require_contains "$CLASS_OUT" "dev/tools/module-boundary-gate/src/config.rs" \
  "module-boundary missing classification names config.rs"
require_contains "$CLASS_OUT" "src/rust/crates/fathomdb-engine/src/lib.rs" \
  "module-boundary missing classification names engine lib.rs"

# 3b. Unsupported cfg predicate
FIX_CFG="$TMPROOT/boundary-unsupported-cfg"
make_boundary_fixture "$FIX_CFG"
printf '\n#[cfg(synthetic_unknown_cfg)]\npub(crate) fn gated() {}\n' >>"$FIX_CFG/src/rust/crates/fathomdb-engine/src/search.rs"
set +e
CFG_OUT="$("$GATE_BIN" --root "$FIX_CFG" 2>&1)"
CFG_RC=$?
set -e
if [ "$CFG_RC" -ne 0 ]; then
  pass "module-boundary unsupported cfg fails non-zero"
else
  fail "module-boundary unsupported cfg unexpectedly exited 0"
fi
require_contains "$CFG_OUT" "unsupported cfg predicate" \
  "module-boundary unsupported cfg preserves existing diagnostic substring"
require_contains "$CFG_OUT" "dev/tools/module-boundary-policy.txt" \
  "module-boundary unsupported cfg names module-boundary-policy.txt"
require_contains "$CFG_OUT" "dev/tools/module-boundary-gate/src/config.rs" \
  "module-boundary unsupported cfg names config.rs"
require_contains "$CFG_OUT" "src/rust/crates/fathomdb-engine/src/lib.rs" \
  "module-boundary unsupported cfg names engine lib.rs"

# 3c. Undeclared configuration feature (config.rs error path)
FIX_FEAT="$TMPROOT/boundary-undeclared-feature"
make_boundary_fixture "$FIX_FEAT"
printf 'configuration-feature nonexistent_feature nf\n' >>"$FIX_FEAT/dev/tools/module-boundary-policy.txt"
set +e
FEAT_OUT="$("$GATE_BIN" --root "$FIX_FEAT" 2>&1)"
FEAT_RC=$?
set -e
if [ "$FEAT_RC" -ne 0 ]; then
  pass "module-boundary undeclared configuration feature fails non-zero"
else
  fail "module-boundary undeclared configuration feature unexpectedly exited 0"
fi
require_contains "$FEAT_OUT" "configuration feature nonexistent_feature is not declared" \
  "module-boundary undeclared feature preserves existing config.rs diagnostic substring"
require_contains "$FEAT_OUT" "dev/tools/module-boundary-policy.txt" \
  "module-boundary undeclared feature names module-boundary-policy.txt"
require_contains "$FEAT_OUT" "dev/tools/module-boundary-gate/src/config.rs" \
  "module-boundary undeclared feature names config.rs"
require_contains "$FEAT_OUT" "src/rust/crates/fathomdb-engine/src/lib.rs" \
  "module-boundary undeclared feature names engine lib.rs"

# 3d. Undeclared Rust source file
FIX_UNDECL="$TMPROOT/boundary-undeclared-file"
make_boundary_fixture "$FIX_UNDECL"
printf 'pub(crate) fn orphan() {}\n' >"$FIX_UNDECL/src/rust/crates/fathomdb-engine/src/orphan_mod.rs"
set +e
UNDECL_OUT="$("$GATE_BIN" --root "$FIX_UNDECL" 2>&1)"
UNDECL_RC=$?
set -e
if [ "$UNDECL_RC" -ne 0 ]; then
  pass "module-boundary undeclared file fails non-zero"
else
  fail "module-boundary undeclared file unexpectedly exited 0"
fi
require_contains "$UNDECL_OUT" "Rust source file is not declared from lib.rs orphan_mod" \
  "module-boundary undeclared file preserves existing diagnostic substring"
require_contains "$UNDECL_OUT" "src/rust/crates/fathomdb-engine/src/lib.rs" \
  "module-boundary undeclared file names engine lib.rs"
require_contains "$UNDECL_OUT" "dev/tools/module-boundary-policy.txt" \
  "module-boundary undeclared file names module-boundary-policy.txt"

if [ "$FAILED" -gt 0 ]; then
  printf '\n%d diagnostic contract check(s) failed\n' "$FAILED" >&2
  exit 1
fi
printf '\nall 0.8.32 R32-07 gate diagnostic contract tests passed\n'
