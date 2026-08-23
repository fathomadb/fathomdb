#!/usr/bin/env bash
# Regression coverage for the narrow long-running CI efficiency changes:
# dependency cache ownership, no one-off Ripgrep install, visible advisory BGE
# skips, serial default-embedder Node execution, and the advisory race report.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CI="$REPO_ROOT/.github/workflows/ci.yml"
DEV_TOOLS_CONTRACT="$REPO_ROOT/scripts/tests/test_dev_environment_tools_contract.sh"
NODE_BIN="${NODE_BIN:-node}"
YAML_MODULE="${YAML_MODULE:-$REPO_ROOT/node_modules/js-yaml}"

PASS=0
FAIL=0
pass() { printf 'PASS  %s\n' "$1"; PASS=$((PASS + 1)); }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAIL=$((FAIL + 1)); }

WORK="$(mktemp -d)"
cleanup() {
  case "$WORK" in
    "${TMPDIR:-/tmp}"/*|/tmp/*) rm -rf "$WORK" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$WORK" >&2 ;;
  esac
}
trap cleanup EXIT

if ! command -v "$NODE_BIN" >/dev/null 2>&1 || [ ! -d "$YAML_MODULE" ]; then
  fail "declared js-yaml tooling is available; run bash scripts/bootstrap.sh"
  exit 1
fi

CONTRACT="$WORK/contract.json"
WARM_BODY="$WORK/warm-body.sh"
"$NODE_BIN" - "$CI" "$YAML_MODULE" "$CONTRACT" "$WARM_BODY" <<'JS'
const fs = require("fs");
const [workflowPath, yamlModule, contractPath, warmBodyPath] = process.argv.slice(2);
const workflow = require(yamlModule).load(fs.readFileSync(workflowPath, "utf8"));
const jobs = workflow.jobs || {};

function steps(jobName) { return (jobs[jobName] || {}).steps || []; }
function actionStep(jobName, action) {
  return steps(jobName).find((step) => String(step.uses || "").startsWith(action)) || {};
}
function namedStep(jobName, name) {
  return steps(jobName).find((step) => step.name === name) || {};
}
function cacheContract(jobName, action) {
  const step = actionStep(jobName, action);
  return {
    cache: String((step.with || {}).cache || ""),
    path: String((step.with || {})["cache-dependency-path"] || "").trim(),
  };
}

const fastRuns = steps("verify-fast").map((step) => String(step.run || ""));
const warm = namedStep("default-embedder-tests", "Warm BGE embedder cache (no-op on cache hit)");
const warmRun = String(warm.run || "");
const nodeRun = String(namedStep(
  "default-embedder-tests",
  "Default-embedder and release-surface TypeScript tests",
).run || "");
const race = namedStep("rust-workspace-race-report", "Parallel Rust workspace race report");
const raceUpload = steps("rust-workspace-race-report").find(
  (step) => String(step.uses || "").startsWith("actions/upload-artifact@"),
) || {};

const contract = {
  fastPython: cacheContract("verify-fast", "actions/setup-python@"),
  fastNode: cacheContract("verify-fast", "actions/setup-node@"),
  heavyPython: cacheContract("verify", "actions/setup-python@"),
  heavyNode: cacheContract("verify", "actions/setup-node@"),
  embedderPython: cacheContract("default-embedder-tests", "actions/setup-python@"),
  embedderNode: cacheContract("default-embedder-tests", "actions/setup-node@"),
  noRipgrepInstall: !fastRuns.some((run) => /apt-get[^\n]*ripgrep|install[^\n]*ripgrep/i.test(run)),
  warmUsesCacheEnv: Boolean(warm.env && warm.env.BGE_CACHE_HIT && warmRun.includes("$BGE_CACHE_HIT")),
  warmWarns: warmRun.includes("::warning title=BGE live-model coverage skipped::"),
  warmSummarizes: warmRun.includes("GITHUB_STEP_SUMMARY") && warmRun.includes("live-model-tests: skipped"),
  warmExportsSkip: warmRun.includes("FATHOMDB_SKIP_NETWORK_TESTS=1") && warmRun.includes("GITHUB_ENV"),
  warmRestoresErrexit: warmRun.includes("set +e") && warmRun.includes("set -e"),
  nodeSerialLoop: nodeRun.includes("for test_file in")
    && nodeRun.includes('node --test "$test_file"')
    && !nodeRun.includes("xargs -P")
    && !nodeRun.includes("parallel ")
    && !nodeRun.includes('node --test "$test_file" &'),
  raceAdvisory: String(race.run || "").includes("set +e")
    && String(race.run || "").includes("::warning title=Rust workspace parallel report::")
    && String(race.run || "").includes("exit 0")
    && raceUpload.if === "always()",
};

fs.writeFileSync(contractPath, JSON.stringify(contract, null, 2));
fs.writeFileSync(warmBodyPath, warmRun);
JS

contract_value() {
  "$NODE_BIN" - "$CONTRACT" "$1" <<'JS'
const fs = require("fs");
const [contractPath, dottedKey] = process.argv.slice(2);
let value = JSON.parse(fs.readFileSync(contractPath, "utf8"));
for (const key of dottedKey.split(".")) value = value[key];
process.stdout.write(String(value));
JS
}

expect_value() {
  local key="$1" expected="$2" description="$3" actual
  actual="$(contract_value "$key")"
  if [ "$actual" = "$expected" ]; then
    pass "$description"
  else
    fail "$description (expected=$expected actual=$actual)"
  fi
}

expect_value noRipgrepInstall true "verify-fast has no package-manager Ripgrep installation"
if grep -Fq 'rg -Fq' "$DEV_TOOLS_CONTRACT"; then
  fail "developer-environment contract still requires Ripgrep for fixed-string matching"
else
  pass "developer-environment contract uses no Ripgrep executable"
fi

expect_value fastPython.cache pip "verify-fast enables pip caching"
expect_value fastPython.path src/python/pyproject.toml "verify-fast keys pip cache from the Python project"
expect_value fastNode.cache npm "verify-fast enables npm caching"
expect_value fastNode.path $'package-lock.json\nsrc/ts/package-lock.json' \
  "verify-fast keys npm cache from root and TypeScript locks"
expect_value heavyPython.cache pip "verify enables pip caching"
expect_value heavyPython.path src/python/pyproject.toml "verify keys pip cache from the Python project"
expect_value heavyNode.cache npm "verify enables npm caching"
expect_value heavyNode.path src/ts/package-lock.json "verify keys npm cache from the TypeScript lock"
expect_value embedderPython.cache pip "default-embedder enables pip caching"
expect_value embedderPython.path src/python/pyproject.toml \
  "default-embedder keys pip cache from the Python project"
expect_value embedderNode.cache npm "default-embedder enables npm caching"
expect_value embedderNode.path src/ts/package-lock.json \
  "default-embedder keys npm cache from the TypeScript lock"

expect_value warmUsesCacheEnv true "BGE warm step receives cache-hit state through its environment"
expect_value warmWarns true "BGE warm failure emits an explicit GitHub warning"
expect_value warmSummarizes true "BGE warm failure records skipped live-model coverage"
expect_value warmExportsSkip true "BGE warm failure exports the network-test skip gate"
expect_value warmRestoresErrexit true "BGE warm step restores fail-fast behavior after Cargo"
expect_value nodeSerialLoop true "default-embedder Node test files remain serial and isolated"
expect_value raceAdvisory true "Rust race report remains independent and advisory"

warm_uses_cache_env="$(contract_value warmUsesCacheEnv)"
warm_warns="$(contract_value warmWarns)"
warm_summarizes="$(contract_value warmSummarizes)"
if [ "$warm_uses_cache_env" = true ] &&
  [ "$warm_warns" = true ] &&
  [ "$warm_summarizes" = true ]; then
  FAKE_BIN="$WORK/bin"
  mkdir -p "$FAKE_BIN"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "fake cargo rc=%s\\n" "${FAKE_CARGO_RC:-0}"' \
    'exit "${FAKE_CARGO_RC:-0}"' >"$FAKE_BIN/cargo"
  chmod +x "$FAKE_BIN/cargo"

  : >"$WORK/success-env"
  : >"$WORK/success-summary"
  set +e
  success_out="$(PATH="$FAKE_BIN:$PATH" FAKE_CARGO_RC=0 BGE_CACHE_HIT=false \
    GITHUB_ENV="$WORK/success-env" GITHUB_STEP_SUMMARY="$WORK/success-summary" \
    bash "$WARM_BODY" 2>&1)"
  success_rc=$?
  set -e
  if [ "$success_rc" -eq 0 ] && [ ! -s "$WORK/success-env" ] &&
    [[ "$success_out" != *"::warning"* ]] && [ ! -s "$WORK/success-summary" ]; then
    pass "successful BGE warm leaves live-model coverage enabled"
  else
    fail "successful BGE warm changed skip state (rc=$success_rc output=$success_out)"
  fi

  : >"$WORK/fail-env"
  : >"$WORK/fail-summary"
  set +e
  fail_out="$(PATH="$FAKE_BIN:$PATH" FAKE_CARGO_RC=73 BGE_CACHE_HIT=false \
    GITHUB_ENV="$WORK/fail-env" GITHUB_STEP_SUMMARY="$WORK/fail-summary" \
    bash "$WARM_BODY" 2>&1)"
  fail_rc=$?
  set -e
  if [ "$fail_rc" -eq 0 ] &&
    grep -Fxq 'FATHOMDB_SKIP_NETWORK_TESTS=1' "$WORK/fail-env" &&
    grep -Fq 'live-model-tests: skipped' "$WORK/fail-summary" &&
    grep -Fq 'warm-cache-exit-code: 73' "$WORK/fail-summary" &&
    [[ "$fail_out" == *"::warning title=BGE live-model coverage skipped::"* ]]; then
    pass "failed BGE warm visibly skips live-model tests without gating"
  else
    fail "failed BGE warm lost advisory evidence (rc=$fail_rc output=$fail_out)"
  fi

  set +e
  PATH="$FAKE_BIN:$PATH" FAKE_CARGO_RC=73 BGE_CACHE_HIT=false \
    GITHUB_ENV="$WORK/unwritable-env" GITHUB_STEP_SUMMARY="$WORK/missing/summary" \
    bash "$WARM_BODY" >"$WORK/write-failure.out" 2>&1
  write_failure_rc=$?
  set -e
  if [ "$write_failure_rc" -ne 0 ]; then
    pass "BGE warm step does not hide failure to record its advisory state"
  else
    fail "BGE warm step hid failure to record its advisory state"
  fi
else
  fail "BGE warm behavior could not be executed because its workflow contract is incomplete"
fi

printf '%s passed, %s failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
