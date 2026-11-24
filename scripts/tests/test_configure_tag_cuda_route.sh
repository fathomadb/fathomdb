#!/usr/bin/env bash
# Unit tests for scripts/release/configure-tag-cuda-route.sh.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TARGET="$REPO_ROOT/scripts/release/configure-tag-cuda-route.sh"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/bin" "$work/state"
cat > "$work/state/group3.json" <<'JSON'
{"id":3,"name":"fathomdb-gpu-release","visibility":"selected","allows_public_repositories":true,"restricted_to_workflows":true,"selected_workflows":["fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main"]}
JSON
cat > "$work/state/repos.json" <<'JSON'
{"total_count":1,"repositories":[{"id":1188097188,"full_name":"fathomadb/fathomdb"}]}
JSON
cat > "$work/state/cuda-policies.json" <<'JSON'
{"total_count":1,"branch_policies":[{"name":"v0.8.31","type":"tag"}]}
JSON
cat > "$work/state/pages-policies.json" <<'JSON'
{"total_count":1,"branch_policies":[{"name":"main","type":"branch"}]}
JSON
cat > "$work/state/cuda-env.json" <<'JSON'
{"protection_rules":[{"type":"required_reviewers","reviewers":[{"type":"User","reviewer":{"id":219899}}]}]}
JSON

cat > "$work/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
STATE="$FAKE_GH_STATE"
method="GET"
if [ "${1:-}" = "api" ]; then shift; fi
if [ "${1:-}" = "-X" ]; then method="$2"; shift 2; fi
endpoint="$1"; shift
case "$method $endpoint" in
  "GET orgs/fathomadb/actions/runner-groups/3")
    cat "$STATE/group3.json" ;;
  "GET orgs/fathomadb/actions/runner-groups/3/repositories")
    cat "$STATE/repos.json" ;;
  "PATCH orgs/fathomadb/actions/runner-groups/3")
    body="$(cat)"
    printf '%s\n' "$body" > "$STATE/last-patch.json"
    # Simulate GitHub's real quirk: omitting selected_repository_ids wipes repos!
    if ! jq -e '(.selected_repository_ids | type == "array" and length > 0)' <<<"$body" >/dev/null; then
      printf '{"total_count":0,"repositories":[]}\n' > "$STATE/repos.json"
    fi
    jq -c '. + {id: 3}' <<<"$body" > "$STATE/group3.json"
    cat "$STATE/group3.json" ;;
  "GET repos/fathomadb/fathomdb/environments/cuda-unmerged-preflight/deployment-branch-policies")
    cat "$STATE/cuda-policies.json" ;;
  "GET repos/fathomadb/fathomdb/environments/github-pages/deployment-branch-policies")
    cat "$STATE/pages-policies.json" ;;
  "POST repos/fathomadb/fathomdb/environments/github-pages/deployment-branch-policies")
    printf '{"total_count":2,"branch_policies":[{"name":"main","type":"branch"},{"name":"release/0.8.31","type":"branch"}]}\n' > "$STATE/pages-policies.json"
    printf '{"id":1}\n' ;;
  "GET repos/fathomadb/fathomdb/environments/cuda-unmerged-preflight")
    cat "$STATE/cuda-env.json" ;;
  "GET repos/fathomadb/fathomdb/git/ref/tags/v0.8.31")
    printf 'HTTP/2.0 404 Not Found\r\n\r\n{"message":"Not Found","status":"404"}\n'
    exit 1 ;;
  *)
    printf 'unexpected gh call: %s %s\n' "$method" "$endpoint" >&2
    exit 99 ;;
esac
SH
chmod +x "$work/bin/gh"

# 1. Post-tag mode must preserve selected_repository_ids and pass check-tag-cuda-route.sh.
PATH="$work/bin:$PATH" FAKE_GH_STATE="$work/state" bash "$TARGET" v0.8.31 >/dev/null
jq -e '.selected_repository_ids == [1188097188]' "$work/state/last-patch.json" >/dev/null
jq -e '.selected_workflows | index("fathomadb/fathomdb/.github/workflows/release.yml@refs/tags/v0.8.31") != null' "$work/state/last-patch.json" >/dev/null

# 2. Empty selected repositories must fail closed before PATCH.
printf '{"total_count":0,"repositories":[]}\n' > "$work/state/repos.json"
if PATH="$work/bin:$PATH" FAKE_GH_STATE="$work/state" bash "$TARGET" v0.8.31 >/dev/null 2>&1; then
  printf 'FAIL: expected configure-tag-cuda-route.sh to refuse empty selected repositories\n' >&2
  exit 1
fi

printf 'PASS test_configure_tag_cuda_route\n'
