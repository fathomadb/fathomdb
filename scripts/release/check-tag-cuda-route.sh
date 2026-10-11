#!/usr/bin/env bash
# Fail before a release tag push if GitHub cannot route its CUDA job.
set -euo pipefail

tag="${1:-}"
if ! [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'check-tag-cuda-route: expected a vX.Y.Z release tag\n' >&2
  exit 2
fi
command -v gh >/dev/null || {
  printf 'check-tag-cuda-route: gh is required to inspect GitHub release routing\n' >&2
  exit 1
}

group_json="$(gh api orgs/fathomadb/actions/runner-groups/3)" || {
  printf 'check-tag-cuda-route: cannot read organization runner group 3; GitHub authentication needs runner-group read access\n' >&2
  exit 1
}
workflow_ref="fathomadb/fathomdb/.github/workflows/release.yml@refs/tags/$tag"
if ! jq -e --arg workflow_ref "$workflow_ref" '
  .name == "fathomdb-gpu-release"
  and .visibility == "selected"
  and .restricted_to_workflows == true
  and (.selected_workflows | index($workflow_ref) != null)
' <<<"$group_json" >/dev/null; then
  printf 'check-tag-cuda-route: GPU runner group does not allow %s\n' "$workflow_ref" >&2
  exit 1
fi

env_json="$(gh api repos/fathomadb/fathomdb/environments/cuda-unmerged-preflight/deployment-branch-policies)" || {
  printf 'check-tag-cuda-route: cannot read cuda-unmerged-preflight deployment policies\n' >&2
  exit 1
}
if ! jq -e --arg tag "$tag" '
  .branch_policies | any(.type == "tag" and .name == $tag)
' <<<"$env_json" >/dev/null; then
  printf 'check-tag-cuda-route: cuda-unmerged-preflight does not allow tag %s\n' "$tag" >&2
  exit 1
fi
printf 'check-tag-cuda-route: %s can reach the GPU runner and protected environment\n' "$tag"
