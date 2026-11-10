#!/usr/bin/env bash
# Check CUDA tag routing or explicit first-tag readiness with GPU approval held.
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

bootstrap="${FATHOMDB_RELEASE_TAG_BOOTSTRAP:-}"
workflow_ref="fathomadb/fathomdb/.github/workflows/release.yml@refs/tags/$tag"
if [ -n "$bootstrap" ]; then
  if [ "$bootstrap" != v0.8.28 ] || [ "$bootstrap" != "$tag" ]; then
    printf 'check-tag-cuda-route: bootstrap marker must match the authorized v0.8.28 tag\n' >&2
    exit 1
  fi
  if ref_response="$(gh api "repos/fathomadb/fathomdb/git/ref/tags/$tag" --include)"; then
    printf 'check-tag-cuda-route: bootstrap requires a confirmed absent remote tag\n' >&2
    exit 1
  fi
  ref_status="${ref_response%%$'\n'*}"
  ref_status="${ref_status%$'\r'}"
  ref_body="$(sed '1,/^[[:space:]]*$/d' <<<"$ref_response")"
  if ! [[ "$ref_status" =~ ^HTTP/[0-9]+(\.[0-9]+)?[[:space:]]404[[:space:]] ]] \
    || ! jq -e 'type == "object" and .message == "Not Found" and .status == "404"' \
      <<<"$ref_body" >/dev/null; then
    printf 'check-tag-cuda-route: bootstrap requires a valid HTTP404 ref response; refusing API/network failure\n' >&2
    exit 1
  fi
  workflow_ref="fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main"
fi

group_json="$(gh api orgs/fathomadb/actions/runner-groups/3)" || {
  printf 'check-tag-cuda-route: cannot read organization runner group 3; GitHub authentication needs runner-group read access\n' >&2
  exit 1
}
if ! jq -e --arg workflow_ref "$workflow_ref" '
  .name == "fathomdb-gpu-release"
  and .visibility == "selected"
  and .restricted_to_workflows == true
  and (.selected_workflows | type == "array" and index($workflow_ref) != null)
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
if [ -n "$bootstrap" ]; then
  environment_json="$(gh api repos/fathomadb/fathomdb/environments/cuda-unmerged-preflight)" || {
    printf 'check-tag-cuda-route: cannot read required GPU environment reviewers\n' >&2
    exit 1
  }
  if ! jq -e '
    .protection_rules | any(
      .type == "required_reviewers"
      and (.reviewers | type == "array" and length > 0 and all(
        (.type == "User" or .type == "Team")
        and (.reviewer.id | type == "number" and floor == . and . > 0)
      ))
    )
  ' <<<"$environment_json" >/dev/null; then
    printf 'check-tag-cuda-route: bootstrap requires actual nonempty GPU environment reviewers\n' >&2
    exit 1
  fi
  printf 'check-tag-cuda-route: bootstrap ready for %s; GPU approval HELD until the normal route check passes\n' "$tag"
else
  printf 'check-tag-cuda-route: %s can reach the GPU runner and protected environment\n' "$tag"
fi
