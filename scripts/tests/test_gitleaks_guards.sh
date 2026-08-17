#!/usr/bin/env bash
# Regression coverage for the staged-index and reachable-history Gitleaks
# guards. The only credential-shaped value exists in a temporary repository and
# is constructed from fragments so it cannot become a tracked secret.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
STAGED_GUARD="$REPO_ROOT/scripts/security/gitleaks-staged.sh"
HISTORY_GUARD="$REPO_ROOT/scripts/security/gitleaks-history.sh"
CURRENT_GUARD="$REPO_ROOT/scripts/security/gitleaks-current.sh"
PRE_COMMIT="$REPO_ROOT/scripts/hooks/pre-commit"
CI="$REPO_ROOT/.github/workflows/ci.yml"
INSTALLER="$REPO_ROOT/scripts/install-gitleaks.sh"
BOOTSTRAP="$REPO_ROOT/scripts/bootstrap.sh"

PASS=0
FAIL=0

pass() {
  printf 'PASS  %s\n' "$1"
  PASS=$((PASS + 1))
}

fail() {
  printf 'FAIL  %s\n' "$1" >&2
  FAIL=$((FAIL + 1))
}

expect_nonzero() {
  local rc="$1" description="$2"
  if [ "$rc" -ne 0 ]; then
    pass "$description"
  else
    fail "$description (expected non-zero exit)"
  fi
}

expect_zero() {
  local rc="$1" description="$2"
  if [ "$rc" -eq 0 ]; then
    pass "$description"
  else
    fail "$description (exit $rc)"
  fi
}

[ -x "$STAGED_GUARD" ] || fail "staged Gitleaks guard exists and is executable"
[ -x "$HISTORY_GUARD" ] || fail "history Gitleaks guard exists and is executable"
[ -x "$CURRENT_GUARD" ] || fail "current-tree Gitleaks guard exists and is executable"
[ -x "$INSTALLER" ] || fail "pinned Gitleaks installer exists and is executable"

if ! command -v gitleaks >/dev/null 2>&1; then
  fail "Gitleaks is installed for guard regression coverage"
  printf '%s passed, %s failed\n' "$PASS" "$FAIL"
  exit 1
fi
GITLEAKS_BIN="$(command -v gitleaks)"

set +e
current_out="$(GITLEAKS_BIN="$GITLEAKS_BIN" "$CURRENT_GUARD" "$REPO_ROOT" 2>&1)"
current_rc=$?
set -e
expect_zero "$current_rc" "current-tree guard accepts the tracked repository"
if [[ "$current_out" == *"synthetic_"* ]]; then
  fail "current-tree guard emits only its fixed safe report"
else
  pass "current-tree guard emits only its fixed safe report"
fi

TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT
FIXTURE="$TMPROOT/repo"
mkdir -p "$FIXTURE"
git -C "$FIXTURE" init -q
git -C "$FIXTURE" config user.email gitleaks-test@example.invalid
git -C "$FIXTURE" config user.name 'Gitleaks Test'

# A temporary local rule makes the fixture deterministic without carrying a
# real credential or depending on a particular upstream default-rule release.
printf '%s\n' \
  'title = "gitleaks guard fixture"' \
  '[[rules]]' \
  'id = "synthetic-secret"' \
  'description = "temporary credential-shaped fixture"' \
  "regex = '''synthetic_[A-Z0-9]{24}'''" >"$FIXTURE/.gitleaks.toml"
printf 'safe=true\n' >"$FIXTURE/safe.txt"
git -C "$FIXTURE" add .gitleaks.toml safe.txt

set +e
GITLEAKS_BIN="$TMPROOT/missing-gitleaks" "$STAGED_GUARD" "$FIXTURE" >"$TMPROOT/missing.out" 2>&1
missing_rc=$?
set -e
expect_nonzero "$missing_rc" "staged guard fails closed when scanner is unavailable"

set +e
GITLEAKS_BIN="$GITLEAKS_BIN" "$STAGED_GUARD" "$FIXTURE" >"$TMPROOT/clean.out" 2>&1
clean_rc=$?
set -e
expect_zero "$clean_rc" "staged guard accepts a clean staged index"

token="$(printf '%s%s%s%s%s%s%s' \
  'synthetic_' 'ABCD' 'EF12' '3456' '7890' 'ABCD' 'EF12')"
printf 'credential=%s\n' "$token" >"$FIXTURE/credential.txt"
git -C "$FIXTURE" add credential.txt

set +e
staged_out="$(GITLEAKS_BIN="$GITLEAKS_BIN" "$STAGED_GUARD" "$FIXTURE" 2>&1)"
staged_rc=$?
set -e
expect_nonzero "$staged_rc" "staged guard rejects a synthetic staged credential"
if [[ "$staged_out" == *"$token"* ]]; then
  fail "staged guard redacts the synthetic credential from output"
else
  pass "staged guard redacts the synthetic credential from output"
fi

git -C "$FIXTURE" commit -qm 'fixture secret history'
set +e
history_out="$(GITLEAKS_BIN="$GITLEAKS_BIN" "$HISTORY_GUARD" "$FIXTURE" 2>&1)"
history_rc=$?
set -e
expect_nonzero "$history_rc" "history guard rejects a reachable synthetic credential"
if [[ "$history_out" == *"$token"* ]]; then
  fail "history guard redacts the synthetic credential from output"
else
  pass "history guard redacts the synthetic credential from output"
fi

if grep -Fq 'gitleaks-staged.sh' "$PRE_COMMIT"; then
  pass "tracked pre-commit hook invokes the staged guard"
else
  fail "tracked pre-commit hook invokes the staged guard"
fi

if grep -Fq 'install-gitleaks.sh' "$BOOTSTRAP"; then
  pass "bootstrap installs the pinned Gitleaks binary"
else
  fail "bootstrap installs the pinned Gitleaks binary"
fi

if python3 - "$CI" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text()
jobs_start = text.find("jobs:\n")
if jobs_start < 0:
    raise SystemExit(1)
jobs_text = text[jobs_start + len("jobs:\n") :]
jobs = list(re.finditer(r"^  ([A-Za-z0-9_-]+):\n", jobs_text, re.M))
for index, job in enumerate(jobs):
    if job.group(1) == "gitleaks":
        end = jobs[index + 1].start() if index + 1 < len(jobs) else len(jobs_text)
        body = jobs_text[job.end() : end]
        break
else:
    raise SystemExit(1)
required = ("fetch-depth: 0", "install-gitleaks.sh", "gitleaks-current.sh", "gitleaks-history.sh")
if any(item not in body for item in required):
    raise SystemExit(1)
if body.find("gitleaks-current.sh") > body.find("gitleaks-history.sh"):
    raise SystemExit(1)
if re.search(r"^    needs:\s*changes\s*$", body, re.M) or re.search(r"^    if:", body, re.M):
    raise SystemExit(1)
PY
then
  pass "always-on CI history guard has full history and no docs-only bypass"
else
  fail "always-on CI history guard has full history and no docs-only bypass"
fi

printf '%s passed, %s failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
