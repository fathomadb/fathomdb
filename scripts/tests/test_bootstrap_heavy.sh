#!/usr/bin/env bash
# Contract for verify's heavy-only bootstrap. This fixture executes the helper
# with controlled Python/npm tools and separately checks its CI ownership.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
HELPER="$REPO_ROOT/scripts/bootstrap-heavy.sh"
CI="$REPO_ROOT/.github/workflows/ci.yml"
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

if [ ! -x "$HELPER" ]; then
  fail "heavy-only bootstrap exists and is executable"
else
  if grep -Fq 'create_venv_with_selected_python .venv' "$HELPER"; then
    pass "heavy bootstrap uses the canonical checkout-venv selector"
  else
    fail "heavy bootstrap uses the canonical checkout-venv selector"
  fi

  FIXTURE="$WORK/repo"
  FAKE_BIN="$WORK/bin"
  CALLS="$WORK/calls"
  mkdir -p "$FIXTURE/src/python" "$FIXTURE/src/ts" "$FAKE_BIN"
  : >"$FIXTURE/src/python/pyproject.toml"
  : >"$FIXTURE/src/ts/package.json"
  git -C "$FIXTURE" init --quiet

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'if [ "$1" != "-m" ] || [ "$2" != "venv" ]; then exit 91; fi' \
    'mkdir -p "$3/bin"' \
    'printf '\''%s\n'\'' '\''#!/usr/bin/env bash'\'' '\''printf "python" >> "$FAKE_CALL_LOG"'\'' '\''printf "|%s" "$@" >> "$FAKE_CALL_LOG"'\'' '\''printf "\\n" >> "$FAKE_CALL_LOG"'\'' > "$3/bin/python"' \
    'chmod +x "$3/bin/python"' >"$FAKE_BIN/python3.13"
  chmod +x "$FAKE_BIN/python3.13"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "npm|%s" "$PWD" >> "$FAKE_CALL_LOG"' \
    'printf "|%s" "$@" >> "$FAKE_CALL_LOG"' \
    'printf "\\n" >> "$FAKE_CALL_LOG"' >"$FAKE_BIN/npm"
  chmod +x "$FAKE_BIN/npm"

  for forbidden in cargo go sudo apt-get lychee actionlint shellcheck gitleaks; do
    printf '%s\n' \
      '#!/usr/bin/env bash' \
      'printf "forbidden|%s\\n" "$(basename "$0")" >> "$FAKE_CALL_LOG"' \
      'exit 97' >"$FAKE_BIN/$forbidden"
    chmod +x "$FAKE_BIN/$forbidden"
  done

  set +e
  helper_out="$(
    cd "$FIXTURE" &&
      PATH="$FAKE_BIN:/usr/bin:/bin" FAKE_CALL_LOG="$CALLS" bash "$HELPER" 2>&1
  )"
  helper_rc=$?
  set -e
  if [ "$helper_rc" -eq 0 ]; then
    pass "heavy bootstrap completes with only its declared toolchain"
  else
    fail "heavy bootstrap completes with only its declared toolchain (rc=$helper_rc output=$helper_out)"
  fi

  if [ -x "$FIXTURE/.venv/bin/python" ]; then
    pass "heavy bootstrap creates a checkout-owned virtualenv"
  else
    fail "heavy bootstrap creates a checkout-owned virtualenv"
  fi
  if grep -Fxq 'python|-m|pip|install|-e|src/python[test]' "$CALLS"; then
    pass "heavy bootstrap installs the Python test extra exactly"
  else
    fail "heavy bootstrap installs the Python test extra exactly"
  fi
  if grep -Fxq 'python|-c|import pytest, hypothesis' "$CALLS"; then
    pass "heavy bootstrap verifies pytest and Hypothesis imports"
  else
    fail "heavy bootstrap verifies pytest and Hypothesis imports"
  fi
  if grep -Fxq "npm|$FIXTURE/src/ts|ci" "$CALLS"; then
    pass "heavy bootstrap installs the locked TypeScript dependencies in src/ts"
  else
    fail "heavy bootstrap installs the locked TypeScript dependencies in src/ts"
  fi
  if [ "$(grep -c '^npm|' "$CALLS" || true)" -eq 1 ]; then
    pass "heavy bootstrap does not install root Markdown dependencies"
  else
    fail "heavy bootstrap does not install root Markdown dependencies"
  fi
  if grep -Eq 'forbidden|\[dev\]|ruff|pyright|--upgrade' "$CALLS"; then
    fail "heavy bootstrap excludes developer-only and unrelated tooling"
  else
    pass "heavy bootstrap excludes developer-only and unrelated tooling"
  fi
fi

if ! command -v "$NODE_BIN" >/dev/null 2>&1 || [ ! -d "$YAML_MODULE" ]; then
  fail "declared js-yaml tooling is available; run bash scripts/bootstrap.sh"
else
  CONTRACT="$WORK/workflow-contract"
  "$NODE_BIN" - "$CI" "$YAML_MODULE" >"$CONTRACT" <<'JS'
const fs = require("fs");
const [workflowPath, yamlModule] = process.argv.slice(2);
const workflow = require(yamlModule).load(fs.readFileSync(workflowPath, "utf8"));
const jobs = workflow.jobs || {};
const runs = (name) => (jobs[name].steps || []).map((step) => String(step.run || ""));
const changeSteps = jobs.changes.steps || [];
const filterStep = changeSteps.find((step) => step.id === "filter") || {};
const filters = String((filterStep.with || {}).filters || "");
const verifyRuns = runs("verify");
const fastRuns = runs("verify-fast");

console.log(`heavy=${verifyRuns.filter((run) => run.includes("bash scripts/bootstrap-heavy.sh")).length}`);
console.log(`heavy-full=${verifyRuns.filter((run) => run.includes("bash scripts/bootstrap.sh")).length}`);
console.log(`fast-full=${fastRuns.filter((run) => run.includes("bash scripts/bootstrap.sh")).length}`);
console.log(`routed=${filters.includes("'scripts/bootstrap-heavy.sh'")}`);
JS

  if grep -Fxq 'heavy=1' "$CONTRACT" && grep -Fxq 'heavy-full=0' "$CONTRACT"; then
    pass "verify uses only the heavy bootstrap"
  else
    fail "verify uses only the heavy bootstrap"
  fi
  if grep -Fxq 'fast-full=1' "$CONTRACT"; then
    pass "verify-fast retains the full developer bootstrap"
  else
    fail "verify-fast retains the full developer bootstrap"
  fi
  if grep -Fxq 'routed=true' "$CONTRACT"; then
    pass "heavy bootstrap changes route through verify_harness"
  else
    fail "heavy bootstrap changes route through verify_harness"
  fi
fi

printf '%s passed, %s failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
