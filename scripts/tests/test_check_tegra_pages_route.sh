#!/usr/bin/env bash
# TDD unit tests for scripts/release/check-tegra-pages-route.sh (branch policy
# plus GitHub Pages pages_build_version same-SHA deduplication guard).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/release/check-tegra-pages-route.sh"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/bin"
cat > "$WORK/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" != "api" ]; then
  echo "unexpected gh subcommand: $*" >&2
  exit 1
fi
endpoint="${2:-}"
case "$endpoint" in
  repos/*/environments/github-pages/deployment-branch-policies)
    printf '%s\n' "${MOCK_POLICIES_JSON}"
    ;;
  repos/*/deployments\?environment=github-pages*)
    printf '%s\n' "${MOCK_DEPLOYMENTS_JSON}"
    ;;
  *)
    echo "unexpected gh api endpoint: $endpoint" >&2
    exit 1
    ;;
esac
EOF
chmod +x "$WORK/bin/gh"

NEW_SHA="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
DEPLOYED_SHA="6854ed767da7cdfb6d0e73cdd3b4abd6d33a408e"
POLICIES_WITH_BRANCH='{"branch_policies":[{"type":"branch","name":"release/0.8.33"}]}'
POLICIES_WITHOUT_BRANCH='{"branch_policies":[{"type":"branch","name":"release/0.8.32"}]}'
DEPLOYMENTS_JSON='[{"id":1,"sha":"6854ed767da7cdfb6d0e73cdd3b4abd6d33a408e","ref":"main"}]'

run_with_mock() {
  PATH="$WORK/bin:$PATH" \
    MOCK_POLICIES_JSON="$1" \
    MOCK_DEPLOYMENTS_JSON="$2" \
    bash "$SCRIPT" "${@:3}"
}

# Arm 1: missing release branch policy fails closed.
rc=0
run_with_mock "$POLICIES_WITHOUT_BRANCH" "$DEPLOYMENTS_JSON" 0.8.33 fathomadb/fathomdb >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 1 ] || { printf 'FAIL [arm1]: expected exit 1, got %s\n' "$rc" >&2; exit 1; }
printf 'PASS arm1 rejects missing github-pages branch policy\n'

# Arm 2: backward-compatible 2-arg call succeeds when branch policy exists.
run_with_mock "$POLICIES_WITH_BRANCH" "$DEPLOYMENTS_JSON" 0.8.33 fathomadb/fathomdb >/dev/null
printf 'PASS arm2 accepts 2-arg call when branch policy exists\n'

# Arm 3: 3-arg call with fresh candidate_sha succeeds.
run_with_mock "$POLICIES_WITH_BRANCH" "$DEPLOYMENTS_JSON" 0.8.33 fathomadb/fathomdb "$NEW_SHA" >/dev/null
printf 'PASS arm3 accepts fresh candidate_sha not yet deployed to github-pages\n'

# Arm 4: 3-arg call with already-deployed SHA fails closed against Pages deduplication.
err_file="$WORK/arm4.err"
rc=0
run_with_mock "$POLICIES_WITH_BRANCH" "$DEPLOYMENTS_JSON" 0.8.33 fathomadb/fathomdb "$DEPLOYED_SHA" >/dev/null 2>"$err_file" || rc=$?
[ "$rc" -eq 1 ] || { printf 'FAIL [arm4]: expected exit 1, got %s\n' "$rc" >&2; exit 1; }
grep -qi 'pages_build_version\|already deployed' "$err_file" || {
  err_body="$(cat "$err_file")"
  printf 'FAIL [arm4]: expected deduplication diagnostic, got:\n%s\n' "$err_body" >&2
  exit 1
}
printf 'PASS arm4 refuses candidate_sha already deployed to github-pages\n'

# Arm 5: malformed candidate_sha returns usage error (exit 2).
rc=0
run_with_mock "$POLICIES_WITH_BRANCH" "$DEPLOYMENTS_JSON" 0.8.33 fathomadb/fathomdb "not-a-sha" >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 2 ] || { printf 'FAIL [arm5]: expected exit 2, got %s\n' "$rc" >&2; exit 1; }
printf 'PASS arm5 rejects malformed candidate_sha\n'
