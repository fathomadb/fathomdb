#!/usr/bin/env bash
# Offline checks for the external CUDA route required before a release tag push.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
guard="$repo_root/scripts/release/check-tag-cuda-route.sh"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

cat >"$fixture/gh" <<'SHIM'
#!/usr/bin/env bash
case "$2" in
  orgs/fathomadb/actions/runner-groups/3) cat "$ROUTE_GROUP_JSON" ;;
  repos/fathomadb/fathomdb/environments/cuda-unmerged-preflight/deployment-branch-policies) cat "$ROUTE_ENV_JSON" ;;
  repos/fathomadb/fathomdb/environments/cuda-unmerged-preflight) cat "$ROUTE_ENV_DETAILS_JSON" ;;
  repos/fathomadb/fathomdb/git/ref/tags/v0.8.28)
    case "$ROUTE_REF_STATE" in
      absent) printf 'HTTP/2.0 404 Not Found\r\nContent-Type: application/json\r\n\r\n{"message":"Not Found","status":"404"}\n'; exit 1 ;;
      present) printf 'HTTP/2.0 200 OK\r\n\r\n{"ref":"refs/tags/v0.8.28"}\n' ;;
      forbidden) printf 'HTTP/2.0 403 Forbidden\r\n\r\n{"message":"Forbidden","status":"403"}\n'; exit 1 ;;
      network) printf 'fixture network failure\n' >&2; exit 1 ;;
      malformed) printf 'HTTP/2.0 404 Not Found\r\n\r\nnot-json\n'; exit 1 ;;
      malformed-status) printf 'HTTP/2.0 404 Not Found\r\n\r\n{"message":"Not Found"}\n'; exit 1 ;;
      success-404) printf 'HTTP/2.0 404 Not Found\r\n\r\n{"message":"Not Found","status":"404"}\n' ;;
      *) exit 2 ;;
    esac ;;
  *) exit 2 ;;
esac
SHIM
chmod +x "$fixture/gh"

cat >"$fixture/group-good.json" <<'JSON'
{"name":"fathomdb-gpu-release","visibility":"selected","restricted_to_workflows":true,"selected_workflows":["fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main","fathomadb/fathomdb/.github/workflows/release.yml@refs/tags/v0.8.27"]}
JSON
cat >"$fixture/group-missing.json" <<'JSON'
{"name":"fathomdb-gpu-release","visibility":"selected","restricted_to_workflows":true,"selected_workflows":["fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main"]}
JSON
cat >"$fixture/env-good.json" <<'JSON'
{"branch_policies":[{"name":"main","type":"branch"},{"name":"v0.8.27","type":"tag"}]}
JSON
cat >"$fixture/env-missing.json" <<'JSON'
{"branch_policies":[{"name":"main","type":"branch"}]}
JSON

run_guard() {
  FATHOMDB_RELEASE_TAG_BOOTSTRAP= PATH="$fixture:$PATH" ROUTE_GROUP_JSON="$1" ROUTE_ENV_JSON="$2" \
    bash "$guard" v0.8.27
}

if run_guard "$fixture/group-good.json" "$fixture/env-good.json" >/dev/null; then
  printf 'PASS exact tag allowed by runner group and environment\n'
else
  printf 'FAIL exact tag route rejected\n' >&2
  exit 1
fi
if run_guard "$fixture/group-missing.json" "$fixture/env-good.json" >/dev/null 2>&1; then
  printf 'FAIL missing runner workflow ref accepted\n' >&2
  exit 1
else
  printf 'PASS missing runner workflow ref rejected\n'
fi
if run_guard "$fixture/group-good.json" "$fixture/env-missing.json" >/dev/null 2>&1; then
  printf 'FAIL missing environment tag policy accepted\n' >&2
  exit 1
else
  printf 'PASS missing environment tag policy rejected\n'
fi

cat >"$fixture/env28-good.json" <<'JSON'
{"branch_policies":[{"name":"main","type":"branch"},{"name":"v0.8.28","type":"tag"}]}
JSON
cat >"$fixture/reviewers-good.json" <<'JSON'
{"protection_rules":[{"type":"required_reviewers","reviewers":[{"type":"User","reviewer":{"id":123}}]}]}
JSON
jq '.selected_workflows=[]' "$fixture/group-missing.json" >"$fixture/no-main.json"
jq '.restricted_to_workflows=false' "$fixture/group-missing.json" >"$fixture/unrestricted.json"
jq '.name="other"' "$fixture/group-missing.json" >"$fixture/wrong-name.json"
jq '.visibility="all"' "$fixture/group-missing.json" >"$fixture/public-group.json"
printf '{"protection_rules":[]}\n' >"$fixture/no-reviewers.json"
printf '{"protection_rules":[{"type":"required_reviewers","reviewers":[]}]}\n' >"$fixture/empty-reviewers.json"
printf '{"protection_rules":[{"type":"required_reviewers","reviewers":[{}]}]}\n' >"$fixture/malformed-reviewers.json"

run_bootstrap() {
  local marker="$1"
  shift
  local -a marker_env=(-u FATHOMDB_RELEASE_TAG_BOOTSTRAP)
  if [ "$marker" != unset ]; then
    marker_env=("FATHOMDB_RELEASE_TAG_BOOTSTRAP=$marker")
  fi
  env "${marker_env[@]}" PATH="$fixture:$PATH" \
    ROUTE_GROUP_JSON="$fixture/group-missing.json" ROUTE_ENV_JSON="$fixture/env28-good.json" \
    ROUTE_ENV_DETAILS_JSON="$fixture/reviewers-good.json" ROUTE_REF_STATE=absent \
    "$@" bash "$guard" v0.8.28
}

if output="$(run_bootstrap v0.8.28)" \
  && [[ "$output" == *"bootstrap ready"* ]] \
  && [[ "$output" == *"GPU approval HELD"* ]] \
  && [[ "$output" != *"can reach the GPU runner"* ]]; then
  printf 'PASS explicit absent-tag bootstrap keeps GPU approval held\n'
else
  printf 'FAIL explicit absent-tag bootstrap rejected or claimed an already-routable tag\n' >&2
  exit 1
fi

reject_bootstrap() {
  local label="$1" marker="$2"
  shift 2
  if run_bootstrap "$marker" "$@" >/dev/null 2>&1; then
    printf 'FAIL bootstrap accepted %s\n' "$label" >&2
    exit 1
  fi
  printf 'PASS bootstrap rejects %s\n' "$label"
}
reject_bootstrap 'unset marker' unset
reject_bootstrap 'wrong marker' v0.8.27
reject_bootstrap 'existing tag' v0.8.28 ROUTE_REF_STATE=present
reject_bootstrap 'non-404 response' v0.8.28 ROUTE_REF_STATE=forbidden
reject_bootstrap 'network failure' v0.8.28 ROUTE_REF_STATE=network
reject_bootstrap 'malformed ref response' v0.8.28 ROUTE_REF_STATE=malformed
reject_bootstrap 'missing ref status' v0.8.28 ROUTE_REF_STATE=malformed-status
reject_bootstrap 'successful malformed ref response' v0.8.28 ROUTE_REF_STATE=success-404
reject_bootstrap 'absent trusted main workflow' v0.8.28 ROUTE_GROUP_JSON="$fixture/no-main.json"
reject_bootstrap 'unrestricted group' v0.8.28 ROUTE_GROUP_JSON="$fixture/unrestricted.json"
reject_bootstrap 'wrong group name' v0.8.28 ROUTE_GROUP_JSON="$fixture/wrong-name.json"
reject_bootstrap 'wrong group visibility' v0.8.28 ROUTE_GROUP_JSON="$fixture/public-group.json"
reject_bootstrap 'missing environment tag policy' v0.8.28 ROUTE_ENV_JSON="$fixture/env-missing.json"
reject_bootstrap 'missing required-reviewer rule' v0.8.28 ROUTE_ENV_DETAILS_JSON="$fixture/no-reviewers.json"
reject_bootstrap 'empty reviewers' v0.8.28 ROUTE_ENV_DETAILS_JSON="$fixture/empty-reviewers.json"
reject_bootstrap 'malformed reviewers' v0.8.28 ROUTE_ENV_DETAILS_JSON="$fixture/malformed-reviewers.json"
