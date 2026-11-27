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
    # Match real GitHub REST API: selected_repository_ids is rejected (HTTP 422)
    # on PATCH, and PATCH clears selected repositories until PUT restores them.
    if jq -e 'has("selected_repository_ids")' <<<"$body" >/dev/null; then
      printf '"selected_repository_ids" is not a permitted key. (HTTP 422)\n' >&2
      exit 1
    fi
    printf '{"total_count":0,"repositories":[]}\n' > "$STATE/repos.json"
    jq -c '. + {id: 3}' <<<"$body" > "$STATE/group3.json"
    cat "$STATE/group3.json" ;;
  "PUT orgs/fathomadb/actions/runner-groups/3/repositories")
    body="$(cat)"
    printf '%s\n' "$body" > "$STATE/last-put-repos.json"
    case "${FAKE_PUT_MODE:-ok}" in
      fail)
        printf 'simulated PUT restore failure (HTTP 502)\n' >&2
        exit 1 ;;
      noop)
        printf '{}\n' ;;
      *)
        if jq -e '.selected_repository_ids == [1188097188]' <<<"$body" >/dev/null; then
          printf '{"total_count":1,"repositories":[{"id":1188097188,"full_name":"fathomadb/fathomdb"}]}\n' > "$STATE/repos.json"
        fi
        printf '{}\n' ;;
    esac ;;
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

cat > "$work/Cargo.toml" <<'TOML'
[workspace.package]
version = "0.8.31"
TOML

# 1. Pre-tag mode adds release branch policy and passes bootstrap check-tag-cuda-route.sh.
PATH="$work/bin:$PATH" FAKE_GH_STATE="$work/state" FATHOMDB_CARGO_TOML="$work/Cargo.toml" \
  bash "$TARGET" --pre-tag v0.8.31 >/dev/null
jq -e '.branch_policies | any(.name == "release/0.8.31" and .type == "branch")' "$work/state/pages-policies.json" >/dev/null

# 2. Post-tag mode must not send selected_repository_ids in PATCH, must restore via PUT, and must pass check-tag-cuda-route.sh.
PATH="$work/bin:$PATH" FAKE_GH_STATE="$work/state" FATHOMDB_CARGO_TOML="$work/Cargo.toml" \
  bash "$TARGET" v0.8.31 >/dev/null
jq -e 'has("selected_repository_ids") | not' "$work/state/last-patch.json" >/dev/null
jq -e '.selected_repository_ids == [1188097188]' "$work/state/last-put-repos.json" >/dev/null
jq -e '.selected_workflows | index("fathomadb/fathomdb/.github/workflows/release.yml@refs/tags/v0.8.31") != null' "$work/state/last-patch.json" >/dev/null

# 3. Post-PATCH PUT failure must fail closed with actionable recovery output.
err_file="$work/put-fail.err"
if PATH="$work/bin:$PATH" FAKE_GH_STATE="$work/state" FAKE_PUT_MODE=fail FATHOMDB_CARGO_TOML="$work/Cargo.toml" \
  bash "$TARGET" v0.8.31 >/dev/null 2>"$err_file"; then
  printf 'FAIL: expected configure-tag-cuda-route.sh to fail closed when PUT restore fails\n' >&2
  exit 1
fi
grep -Fq 'restore selected_repository_ids=[1188097188] manually' "$err_file"

# 4. Post-PATCH PUT returning HTTP 200 without reattaching fathomadb/fathomdb must fail closed before check-tag-cuda-route.sh.
printf '{"total_count":1,"repositories":[{"id":1188097188,"full_name":"fathomadb/fathomdb"}]}\n' > "$work/state/repos.json"
noop_err="$work/put-noop.err"
if PATH="$work/bin:$PATH" FAKE_GH_STATE="$work/state" FAKE_PUT_MODE=noop FATHOMDB_CARGO_TOML="$work/Cargo.toml" \
  bash "$TARGET" v0.8.31 >/dev/null 2>"$noop_err"; then
  printf 'FAIL: expected configure-tag-cuda-route.sh to fail closed when PUT leaves repo detached\n' >&2
  exit 1
fi
grep -Fq 'post-PATCH verification lost fathomadb/fathomdb from runner group 3' "$noop_err"

# 5. Empty selected repositories must fail closed before PATCH.
printf '{"total_count":0,"repositories":[]}\n' > "$work/state/repos.json"
if PATH="$work/bin:$PATH" FAKE_GH_STATE="$work/state" FATHOMDB_CARGO_TOML="$work/Cargo.toml" \
  bash "$TARGET" v0.8.31 >/dev/null 2>&1; then
  printf 'FAIL: expected configure-tag-cuda-route.sh to refuse empty selected repositories\n' >&2
  exit 1
fi

printf 'PASS test_configure_tag_cuda_route\n'
