#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
VERIFY="$ROOT/scripts/verify-release-python-wheel.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

if [ ! -x "$VERIFY" ]; then
  fail "release wheel verifier is missing or not executable"
fi

mkdir -p "$TMP/bin" "$TMP/primary/fathomdb"
cat >"$TMP/bin/maturin" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--out" ]; then out="$2"; shift 2; else shift; fi
done
if [ "${FAKE_NO_WHEEL:-0}" != 1 ]; then
  mkdir -p "$out"
  : >"$out/fathomdb-0.8.26-cp312-abi3-linux_x86_64.whl"
fi
SH
chmod +x "$TMP/bin/maturin"

cat >"$TMP/fake-python" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = "-m" ] && [ "${2:-}" = "venv" ]; then
  venv="$3"
  mkdir -p "$venv/bin" "$venv/lib/python3.12/site-packages/fathomdb"
  cp "$0" "$venv/bin/python"
  exit 0
fi
if [ "${1:-}" = "-m" ] && [ "${2:-}" = "pip" ]; then exit 0; fi
if [ -n "${FATHOMDB_VERIFY_REPORT:-}" ]; then
  venv="${FAKE_VENV:?}"
  module="${FAKE_MODULE:-$venv/lib/python3.12/site-packages/fathomdb/__init__.py}"
  native="${FAKE_NATIVE:-$venv/lib/python3.12/site-packages/fathomdb/_fathomdb.so}"
  if [ "${FAKE_SKIP_PROFILE:-0}" = 1 ]; then
    printf '%s\n%s\n%s\n' "$module" "$native" "${FAKE_EDITABLE:-false}" >"$FATHOMDB_VERIFY_REPORT"
  else
    printf '%s\n%s\n%s\nfrozen-evidence-profile-v1\n' \
      "$module" "$native" "${FAKE_EDITABLE:-false}" >"$FATHOMDB_VERIFY_REPORT"
  fi
  if [ -n "${FATHOMDB_SLICE50_EVIDENCE_REPORT:-}" ]; then
    python3 - "$FATHOMDB_SLICE50_EVIDENCE_REPORT" <<'PY'
import json
import sys

rows = []
for seed in ("explicit", "query"):
    for direction in ("outgoing", "incoming", "both"):
        for depth in (1, 2):
            rows.append({
                "seed": seed,
                "direction": direction,
                "depth": depth,
                "hop_count": depth,
                "target_index": 0,
                "target_ref": "target",
                "terminal_ref": "edge",
                "resolved_target_revision": "target-r1",
                "resolved_target_logical_id": "target",
                "resolved_edge_revision": "edge-r1",
                "resolved_edge_class": "edge",
                "resolved_edge_kind": "supports",
                "resolved_edge_from": "root",
                "resolved_edge_to": "target",
                "edge_source": "actuated" if not rows else "ordinary",
                "route_provenance": ["seed", "root", "target", "supports", "outgoing"],
                "intrinsic_evidence": ["target-r1", "edge-r1"],
            })
open(sys.argv[1], "w", encoding="utf-8").write(json.dumps({
    "schema_version": "fathomdb.slice50-graph-evidence/v1",
    "rows": rows,
    "schema_33_refusal": {
        "expected_schema": 33,
        "supported_schema": 34,
        "outcome": "typed_refusal",
        "before": [{"name": "schema-33.sqlite", "size": 1, "sha256": "a" * 64}],
        "after": [{"name": "schema-33.sqlite", "size": 1, "sha256": "a" * 64}],
    },
}))
PY
  fi
  printf 'frozen evidence wheel profile: ok\n'
  exit 0
fi
exit 0
SH
chmod +x "$TMP/fake-python"

run_case() {
  name="$1"; shift
  wheel="$TMP/wheel-$name"
  venv="$TMP/venv-$name"
  set +e
  output="$(env PATH="$TMP/bin:$PATH" FAKE_VENV="$venv" "$@" \
    "$VERIFY" --python "$TMP/fake-python" --wheel-dir "$wheel" --venv-dir "$venv" 2>&1)"
  rc=$?
  set -e
}

run_case success env
[ "$rc" -eq 0 ] || fail "valid isolated wheel fixture passes: $output"
pass "valid isolated wheel fixture passes"

run_case missing env FAKE_NO_WHEEL=1
[ "$rc" -ne 0 ] && grep -q 'exactly one wheel' <<<"$output" \
  || fail "missing wheel is rejected: $output"
pass "missing wheel is rejected"

run_case primary-module env FAKE_MODULE="$TMP/primary/fathomdb/__init__.py"
[ "$rc" -ne 0 ] && grep -q 'module escaped fresh venv' <<<"$output" \
  || fail "primary-tree Python module is rejected: $output"
pass "primary-tree Python module is rejected"

run_case primary-native env FAKE_NATIVE="$TMP/primary/fathomdb/_fathomdb.so"
[ "$rc" -ne 0 ] && grep -q 'native module escaped fresh venv' <<<"$output" \
  || fail "primary-tree native module is rejected: $output"
pass "primary-tree native module is rejected"

run_case editable env FAKE_EDITABLE=true
[ "$rc" -ne 0 ] && grep -q 'editable install' <<<"$output" \
  || fail "editable installation is rejected: $output"
pass "editable installation is rejected"

run_case skipped-profile env FAKE_SKIP_PROFILE=1
[ "$rc" -ne 0 ] && grep -q 'invalid wheel provenance report' <<<"$output" \
  || fail "skipped frozen evidence profile is rejected: $output"
pass "skipped frozen evidence profile is rejected"

printf '\nAll release wheel verifier tests passed\n'
