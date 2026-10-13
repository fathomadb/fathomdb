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
  PATH="$fixture:$PATH" ROUTE_GROUP_JSON="$1" ROUTE_ENV_JSON="$2" \
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
