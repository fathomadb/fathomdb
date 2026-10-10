#!/usr/bin/env bash
# Contract for the installed Jetson Tegra Node package smoke
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decisions 6 and 8): the
# smoke installs the packed packages into a fresh consumer with lifecycle
# scripts disabled, proves the Tegra addon was the one loaded, runs every
# embed/rerank policy row, verifies the GPU witness and the heap-growth
# regression, and never reports a skipped or missing row as a pass.
#
# The live smoke runs only on a classic Jetson Orin. These checks run on any
# host: argument handling and prerequisite failures fail before any host
# probe, and the verdict is exercised against retained-evidence fixtures.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SMOKE="$REPO_ROOT/scripts/release/smoke/smoke-tegra-node-package.sh"
ROW="$REPO_ROOT/scripts/release/smoke/tegra_node_package_row.mjs"
VERDICT="$REPO_ROOT/scripts/release/smoke/tegra_node_package_verdict.py"
CANDIDATE='0123456789abcdef0123456789abcdef01234567'
DIGEST='0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'
FAILED=0

WORK="$(mktemp -d)"
cleanup() {
  case "$WORK" in
    "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$WORK" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$WORK" >&2 ;;
  esac
}
trap cleanup EXIT
pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

for file in "$SMOKE" "$ROW" "$VERDICT"; do
  if [ -f "$file" ]; then
    pass "smoke component exists: ${file#"$REPO_ROOT"/}"
  else
    fail "smoke component is absent: ${file#"$REPO_ROOT"/}"
  fi
done
if [ "$FAILED" -ne 0 ]; then
  printf '%s Tegra Node smoke contract check(s) failed\n' "$FAILED" >&2
  exit 1
fi
if [ -x "$SMOKE" ]; then
  pass 'smoke is executable'
else
  fail 'smoke must be executable'
fi

# --- Argument handling: every malformed invocation is a usage error ---------
printf 'not a tarball\n' >"$WORK/main.tgz"
printf 'not a tarball\n' >"$WORK/tegra.tgz"
PAGES_URL='https://fathomadb.github.io/fathomdb/tegra/packages/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz'

expect_usage_error() {
  local label="$1" status
  shift
  set +e
  bash "$SMOKE" "$@" >"$WORK/usage.out" 2>&1
  status=$?
  set -e
  if [ "$status" -eq 2 ]; then
    pass "$label"
  else
    fail "$label (exit $status)"
    cat "$WORK/usage.out" >&2
  fi
}
expect_usage_error 'no arguments is a usage error'
expect_usage_error 'an unknown flag is a usage error' --source local --skip-heap
expect_usage_error 'an unknown source is a usage error' \
  --source registry --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e1"
expect_usage_error 'local source requires both tarballs' \
  --source local --main-tarball "$WORK/main.tgz" --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e2"
expect_usage_error 'candidate SHA must be 40 lowercase hex' \
  --source local --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha ABC --evidence-dir "$WORK/e3"
expect_usage_error 'evidence directory is required' \
  --source local --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha "$CANDIDATE"
expect_usage_error 'local source rejects Pages-only arguments' \
  --source local --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e4" --pages-url "$PAGES_URL"
expect_usage_error 'pages source rejects local tarballs' \
  --source pages --version 0.8.27 --pages-url "$PAGES_URL" --tegra-sha256 "$DIGEST" \
  --pages-run 123 --main-tarball "$WORK/main.tgz" \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e5"
expect_usage_error 'pages source accepts only the exact first-party tarball URL' \
  --source pages --version 0.8.27 --tegra-sha256 "$DIGEST" --pages-run 123 \
  --pages-url 'https://example.invalid/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz' \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e6"
expect_usage_error 'pages URL must name the requested version' \
  --source pages --version 0.8.28 --tegra-sha256 "$DIGEST" --pages-run 123 \
  --pages-url "$PAGES_URL" --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e7"
expect_usage_error 'pages source requires a 64-hex expected tarball SHA-256' \
  --source pages --version 0.8.27 --tegra-sha256 not-a-digest --pages-run 123 \
  --pages-url "$PAGES_URL" --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e8"
expect_usage_error 'pages source requires a numeric Pages deployment run' \
  --source pages --version 0.8.27 --tegra-sha256 "$DIGEST" --pages-run latest \
  --pages-url "$PAGES_URL" --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e9"
expect_usage_error 'pages version must be a plain release version' \
  --source pages --version 0.8.27+tegra --tegra-sha256 "$DIGEST" --pages-run 123 \
  --pages-url "$PAGES_URL" --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e10"

# --- Prerequisites fail closed before any host probe or install -------------
run_smoke_expect_failure() {
  local label="$1" needle="$2" status
  shift 2
  set +e
  "$@" >"$WORK/prereq.out" 2>&1
  status=$?
  set -e
  if [ "$status" -ne 0 ] && grep -Fq -- "$needle" "$WORK/prereq.out"; then
    pass "$label"
  else
    fail "$label (exit $status)"
    cat "$WORK/prereq.out" >&2
  fi
}
mkdir -p "$WORK/existing-evidence"
run_smoke_expect_failure 'an existing evidence directory is refused' 'evidence path must be new' \
  bash "$SMOKE" --source local --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/existing-evidence"
run_smoke_expect_failure 'an absent local tarball is refused' 'tarball is absent' \
  bash "$SMOKE" --source local --main-tarball "$WORK/absent.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e11"
run_smoke_expect_failure 'a missing offline embedder model cache fails, never skips' \
  'offline model cache is absent' \
  env FATHOMDB_TEGRA_HF_HOME="$WORK/no-hf-cache" \
  bash "$SMOKE" --source local --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e12"
mkdir -p "$WORK/hf-cache"
run_smoke_expect_failure 'a missing offline reranker cache fails, never skips' \
  'offline reranker cache is absent' \
  env FATHOMDB_TEGRA_HF_HOME="$WORK/hf-cache" FATHOMDB_TEGRA_RERANKER_CACHE="$WORK/no-reranker" \
  bash "$SMOKE" --source local --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e13"

# --- Structural guarantees of the live path ---------------------------------
contains() {
  local label="$1" needle="$2"
  if grep -Fq -- "$needle" "$SMOKE"; then pass "$label"; else fail "$label (missing: $needle)"; fi
}
absent() {
  local label="$1" needle="$2"
  if grep -Fq -- "$needle" "$SMOKE"; then fail "$label (unexpected: $needle)"; else pass "$label"; fi
}
contains 'smoke hardens bash' 'set -euo pipefail'
contains 'every npm install disables lifecycle scripts' '--ignore-scripts'
npm_install_lines="$(grep -nE '(^|[^_[:alnum:]])npm(_clean)?[[:space:]]+(install|ci|i)([[:space:]]|$)' "$SMOKE" || true)"
if [ -z "$npm_install_lines" ]; then
  fail 'smoke installs through npm'
else
  unscripted="$(grep -v -- '--ignore-scripts' <<<"$npm_install_lines" || true)"
  if [ -z "$unscripted" ]; then
    pass 'no npm install runs lifecycle scripts'
  else
    fail "npm install without --ignore-scripts: $unscripted"
  fi
fi
contains 'npm user configuration cannot inject a registry or scripts' 'NPM_CONFIG_USERCONFIG=/dev/null'
contains 'local installs never reach a registry' '--offline'
contains 'CUDA library search paths are cleared for loadability' '-u LD_LIBRARY_PATH'
contains 'preloaded libraries are cleared' '-u LD_PRELOAD'
contains 'consumer lives outside the source tree' 'consumer must be outside the source tree'
contains 'a generic package is co-installed to prove selection' 'generic-coinstalled'
contains 'every policy row runs in a fresh process' 'tegra_node_package_row.mjs'
for row in 'embed cpu' 'embed auto' 'embed cuda:0' 'rerank cpu' 'rerank auto' 'rerank cuda:0'; do
  contains "smoke runs the $row row" "run_row $row"
done
contains 'forced CUDA embed requests the allocation witness' 'FATHOMDB_GPU_ALLOCATION_WITNESS=1'
contains 'rerank rows select the reranker policy' 'FATHOMDB_RERANK_DEVICE='
contains 'witness is verified against nvidia-smi' 'verify-tegra-gpu-witness.py'
contains 'witness verifier correlates the device UUID' '--nvidia-smi'
contains 'heap-growth regression runs against the installed package' 'test_tegra_node_early_cuinit.sh'
contains 'heap-growth regression cannot skip' 'FATHOMDB_REQUIRE_LIVE=1'
# shellcheck disable=SC2016 # the needle is the smoke's literal, unexpanded text
contains 'heap-growth regression targets the installed main package' \
  'FATHOMDB_TEGRA_NODE_PACKAGE="$CONSUMER/node_modules/fathomdb"'
contains 'pages source checks the downloaded SHA-256' 'downloaded tarball SHA-256 differs'
contains 'the final verdict is computed from the retained evidence' 'tegra_node_package_verdict.py'
for forbidden in 'npm publish' 'gh release' 'gh workflow' 'twine upload' 'cargo publish' 'deploy-pages' 'npm_config_registry='; do
  absent "smoke cannot publish or redirect: $forbidden" "$forbidden"
done
if grep -Eq 'postinstall' "$SMOKE"; then
  fail 'smoke must not rely on a postinstall step'
else
  pass 'smoke has no postinstall dependency'
fi

# --- Verdict over retained evidence -----------------------------------------
# A complete passing evidence fixture, then single mutations that must each
# turn the verdict into a failure naming the defect.
make_fixture() {
  local dir="$1" source="$2"
  python3 - "$dir" "$source" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
source = sys.argv[2]
node_sha = "a" * 64
rows = root / "rows"
rows.mkdir(parents=True)
consumers = {"tegra-only": "absent", "generic-coinstalled": "decoy"} if source == "local" else {"pages": "registry"}
consumer_roots = {name: f"/tmp/fixture-consumer-{name}" for name in consumers}
primary = "tegra-only" if source == "local" else "pages"
context = {
    "schema": "fathomdb.tegra-node-package-smoke-context/v1",
    "source": source,
    "candidate_sha": "0123456789abcdef0123456789abcdef01234567",
    "version": "0.8.27",
    "tegra_node_sha256": node_sha,
    "primary_consumer": primary,
    "consumers": {
        name: {"root": consumer_roots[name], "generic": generic}
        for name, generic in consumers.items()
    },
}
(root / "context.json").write_text(json.dumps(context, indent=2) + "\n")

def addon(consumer):
    return f"{consumer_roots[consumer]}/node_modules/fathomdb-linux-arm64-gnu-tegra/fathomdb.linux-arm64-gnu.node"

def base(consumer, component, policy):
    return {
        "schema": "fathomdb.tegra-node-package-row/v1",
        "consumer": consumer,
        "consumer_root": consumer_roots[consumer],
        "component": component,
        "policy": policy,
        "outcome": "completed",
        "loaded_native_addons": [addon(consumer)],
        "loaded_native_sha256": {addon(consumer): node_sha},
        "embedder_events": ["DefaultEmbedderCacheHit"],
        "error": None,
    }

def resolution(policy, effective):
    return {"requested_policy": policy, "cuda_compiled": True, "effective_device": effective}

for consumer in consumers:
    row = base(consumer, "select", None)
    (rows / f"{consumer}-select.json").write_text(json.dumps(row) + "\n")
for component in ("embed", "rerank"):
    for policy, effective in (("cpu", "cpu"), ("auto", "cuda"), ("cuda:0", "cuda")):
        row = base(primary, component, policy)
        row["resolution"] = resolution(policy, effective)
        if component == "embed":
            row["embedding_dim"] = 384
            row["witness_present"] = policy == "cuda:0"
        else:
            row["ce_scores"] = [0.25, -1.5]
            row["passages"] = 2
        name = f"{component}-{policy.replace(':', '')}"
        (rows / f"{name}.json").write_text(json.dumps(row) + "\n")
(root / "witness-verification.rc").write_text("0\n")
(root / "witness-verification.log").write_text("tegra-gpu-witness: ok\n")
(root / "heap-growth.rc").write_text("0\n")
(root / "heap-growth.log").write_text(
    "".join(f"PASS  run {i}/3: import first, 1000000 heap objects, forced cuda:0 open and embed\n" for i in (1, 2, 3))
)
PY
}

run_verdict() {
  local dir="$1"
  python3 "$VERDICT" verdict --evidence-dir "$dir" >"$WORK/verdict.out" 2>&1
}

for source in local pages; do
  make_fixture "$WORK/good-$source" "$source"
  if run_verdict "$WORK/good-$source"; then
    if python3 -c 'import json,sys; s=json.load(open(sys.argv[1])); assert s["outcome"] == "PASS", s' \
      "$WORK/good-$source/summary.json"; then
      pass "complete $source evidence yields a PASS summary"
    else
      fail "complete $source evidence must record outcome PASS in summary.json"
    fi
  else
    fail "complete $source evidence must pass the verdict"
    cat "$WORK/verdict.out" >&2
  fi
done

# Each mutation is a short Python edit applied to a fresh good fixture.
expect_verdict_failure() {
  local label="$1" needle="$2" mutation="$3" dir
  dir="$WORK/mut-$((RANDOM))-$((RANDOM))"
  make_fixture "$dir" "${SOURCE:-local}"
  python3 - "$dir" <<PY
import json
from pathlib import Path
root = Path(__import__("sys").argv[1])
rows = root / "rows"
def load(name):
    return json.loads((rows / name).read_text())
def save(name, value):
    (rows / name).write_text(json.dumps(value) + "\n")
$mutation
PY
  if run_verdict "$dir"; then
    fail "$label (verdict passed)"
    cat "$WORK/verdict.out" >&2
  elif grep -Fq -- "$needle" "$WORK/verdict.out"; then
    if python3 -c 'import json,sys; s=json.load(open(sys.argv[1])); assert s["outcome"] == "FAIL", s' \
      "$dir/summary.json" 2>/dev/null; then
      pass "$label"
    else
      fail "$label (summary.json must record outcome FAIL)"
    fi
  else
    fail "$label (diagnostic lacks: $needle)"
    cat "$WORK/verdict.out" >&2
  fi
}

expect_verdict_failure 'a missing forced-CUDA rerank row fails' 'rerank-cuda0: row is missing' \
  '(rows / "rerank-cuda0.json").unlink()'
expect_verdict_failure 'a missing embed CPU row fails' 'embed-cpu: row is missing' \
  '(rows / "embed-cpu.json").unlink()'
expect_verdict_failure 'forced CUDA embedding on CPU is a fallback, not a pass' 'embed-cuda0: effective device cpu' \
  'r = load("embed-cuda0.json"); r["resolution"]["effective_device"] = "cpu"; save("embed-cuda0.json", r)'
expect_verdict_failure 'forced CUDA reranking on CPU is a fallback, not a pass' 'rerank-cuda0: effective device cpu' \
  'r = load("rerank-cuda0.json"); r["resolution"]["effective_device"] = "cpu"; save("rerank-cuda0.json", r)'
expect_verdict_failure 'auto policy must resolve CUDA on the Jetson' 'embed-auto: effective device cpu' \
  'r = load("embed-auto.json"); r["resolution"]["effective_device"] = "cpu"; save("embed-auto.json", r)'
expect_verdict_failure 'cpu policy must stay on CPU' 'rerank-cpu: effective device cuda' \
  'r = load("rerank-cpu.json"); r["resolution"]["effective_device"] = "cuda"; save("rerank-cpu.json", r)'
expect_verdict_failure 'a row that raised is not a pass' 'embed-cuda0: row did not complete' \
  'r = load("embed-cuda0.json"); r["outcome"] = "error"; r["error"] = {"kind": "cuda_probe_failed"}; save("embed-cuda0.json", r)'
expect_verdict_failure 'forced CUDA embedding must carry a witness' 'embed-cuda0: no GPU allocation witness' \
  'r = load("embed-cuda0.json"); r["witness_present"] = False; save("embed-cuda0.json", r)'
expect_verdict_failure 'an identity rerank without cross-encoder scores is not a pass' \
  'rerank-auto: reranker produced no cross-encoder score' \
  'r = load("rerank-auto.json"); r["ce_scores"] = [None, None]; save("rerank-auto.json", r)'
expect_verdict_failure 'a model download is not an offline cache pass' 'embed-auto: default embedder was downloaded' \
  'r = load("embed-auto.json"); r["embedder_events"] = ["DefaultEmbedderDownload"]; save("embed-auto.json", r)'
expect_verdict_failure 'the generic addon being loaded fails selection' 'generic-coinstalled-select: loaded' \
  'r = load("generic-coinstalled-select.json"); g = r["consumer_root"] + "/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node"; r["loaded_native_addons"] = [g]; r["loaded_native_sha256"] = {g: "b" * 64}; save("generic-coinstalled-select.json", r)'
expect_verdict_failure 'loaded addon bytes must be the packed Tegra binary' 'embed-cpu: loaded addon SHA-256' \
  'r = load("embed-cpu.json"); p = r["loaded_native_addons"][0]; r["loaded_native_sha256"] = {p: "c" * 64}; save("embed-cpu.json", r)'
expect_verdict_failure 'local evidence must include the generic co-installed consumer' \
  'generic-coinstalled-select: row is missing' \
  '(rows / "generic-coinstalled-select.json").unlink()'
expect_verdict_failure 'an unverified witness fails' 'witness verification exited 1' \
  '(root / "witness-verification.rc").write_text("1\n")'
expect_verdict_failure 'an absent witness verification fails' 'witness verification was not recorded' \
  '(root / "witness-verification.rc").unlink()'
expect_verdict_failure 'a skipped heap-growth regression is never a pass' 'heap-growth: SKIP' \
  '(root / "heap-growth.log").write_text("SKIP  tegra-node-early-cuinit: Linux aarch64 only\n")'
expect_verdict_failure 'a heap-growth run with no PASS line is not a pass' 'heap-growth: no passing run' \
  '(root / "heap-growth.log").write_text("")'
expect_verdict_failure 'a failed heap-growth regression fails' 'heap-growth: exited 1' \
  '(root / "heap-growth.rc").write_text("1\n")'
expect_verdict_failure 'a partial heap-growth run set fails' 'heap-growth: 2 of 3 runs passed' \
  '(root / "heap-growth.log").write_text("PASS  run 1/3: x\nPASS  run 2/3: x\n")'
expect_verdict_failure 'a context without a candidate SHA fails' 'context: candidate SHA' \
  'c = json.loads((root / "context.json").read_text()); c["candidate_sha"] = "dev"; (root / "context.json").write_text(json.dumps(c))'
SOURCE=pages expect_verdict_failure 'pages evidence needs its forced-CUDA embed row' 'embed-cuda0: row is missing' \
  '(rows / "embed-cuda0.json").unlink()'

# The witness extractor writes the verifier's canonical snake_case record.
mkdir -p "$WORK/witness/rows"
python3 - "$WORK/witness/rows/embed-cuda0.json" <<'PY'
import json
import sys

witness = {
    "schema": "fathomdb.tegra-gpu-allocation-witness/v1",
    "soleGpuConsumerPrecondition": "p",
    "deviceOrdinalRequested": 0,
    "deviceOrdinalActual": 0,
    "deviceUuid": "bbbe9f37-7028-556a-930b-54e5f3b67a82",
    "deviceName": "Orin (nvgpu)",
    "computeCapability": "8.7",
    "freeBeforeBytes": 10,
    "freeAfterBytes": 5,
    "totalBytes": 20,
    "deltaBytes": 5,
    "deltaFloorBytes": 1,
    "controlAllocationRequestBytes": 1,
    "controlBlockCount": 1,
    "controlFreeBeforeBytes": 4,
    "controlFreeAfterBytes": 3,
    "controlDeltaBytes": 1,
    "embeddedVectorDim": 384,
}
open(sys.argv[1], "w").write(json.dumps({"witness": witness}) + "\n")
PY
if python3 "$VERDICT" witness --evidence-dir "$WORK/witness" --out "$WORK/witness/w.json" \
  && python3 - "$WORK/witness/w.json" <<'PY'
import json
import sys

raw = open(sys.argv[1], "rb").read()
value = json.loads(raw)
assert raw == json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode() + b"\n"
assert value["sole_gpu_consumer_precondition"] == "p"
assert value["control_delta_bytes"] == 1 and "controlDeltaBytes" not in value
PY
then
  pass 'witness extractor writes canonical snake_case JSON for the verifier'
else
  fail 'witness extractor must write canonical snake_case JSON'
fi
rm -f "$WORK/witness/rows/embed-cuda0.json"
printf '{"witness": null}\n' >"$WORK/witness/rows/embed-cuda0.json"
if python3 "$VERDICT" witness --evidence-dir "$WORK/witness" --out "$WORK/witness/none.json" >/dev/null 2>&1; then
  fail 'witness extractor must fail when the forced-CUDA row carries no witness'
elif [ -e "$WORK/witness/none.json" ]; then
  fail 'witness extractor must not write a record when there is no witness'
else
  pass 'witness extractor fails closed without a witness'
fi

if [ "$FAILED" -ne 0 ]; then
  printf '%s Tegra Node smoke contract check(s) failed\n' "$FAILED" >&2
  exit 1
fi
printf 'Tegra Node smoke contract: all checks passed\n'
