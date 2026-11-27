#!/usr/bin/env bash
# Safely configure GitHub deployment policies and runner group 3 for a vX.Y.Z
# release tag, restoring selected_repository_ids via PUT .../repositories after
# PATCH (GitHub's PATCH endpoint rejects selected_repository_ids with HTTP 422
# and may clear selected repositories when visibility=selected).
set -euo pipefail

usage() {
  cat <<'USAGE' >&2
Usage:
  scripts/release/configure-tag-cuda-route.sh --pre-tag <vX.Y.Z>
  scripts/release/configure-tag-cuda-route.sh <vX.Y.Z>

Modes:
  --pre-tag <vX.Y.Z>  Add tag policy <vX.Y.Z> to cuda-unmerged-preflight and
                      branch policy release/<X.Y.Z> to github-pages, then run
                      bootstrap check-tag-cuda-route.sh.
  <vX.Y.Z>            Post-tag: add release.yml@refs/tags/<vX.Y.Z> to org
                      runner group 3, restore selected_repository_ids via PUT
                      if cleared by PATCH, verify fathomadb/fathomdb remains
                      attached, and run marker-free check-tag-cuda-route.sh.
USAGE
}

mode="post-tag"
if [ "${1:-}" = "--pre-tag" ]; then
  mode="pre-tag"
  shift
elif [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
  usage
  exit 0
fi

tag="${1:-}"
if [ "$#" -ne 1 ] || ! [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  usage
  exit 2
fi

for tool in gh jq; do
  command -v "$tool" >/dev/null 2>&1 || {
    printf 'configure-tag-cuda-route: required tool not found: %s\n' "$tool" >&2
    exit 1
  }
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ver="${tag#v}"
release_branch="release/${ver}"

ensure_env_policy() {
  local env_name="$1"
  local policy_name="$2"
  local policy_type="$3"
  local existing
  existing="$(gh api "repos/fathomadb/fathomdb/environments/${env_name}/deployment-branch-policies")"
  if jq -e --arg name "$policy_name" --arg type "$policy_type" \
    '.branch_policies | any(.name == $name and .type == $type)' <<<"$existing" >/dev/null; then
    printf 'configure-tag-cuda-route: %s already allows %s %s\n' "$env_name" "$policy_type" "$policy_name"
  else
    gh api -X POST "repos/fathomadb/fathomdb/environments/${env_name}/deployment-branch-policies" \
      -f "name=${policy_name}" -f "type=${policy_type}" >/dev/null
    printf 'configure-tag-cuda-route: added %s %s to %s\n' "$policy_type" "$policy_name" "$env_name"
  fi
}

if [ "$mode" = "pre-tag" ]; then
  ensure_env_policy "cuda-unmerged-preflight" "$tag" "tag"
  ensure_env_policy "github-pages" "$release_branch" "branch"
  FATHOMDB_RELEASE_TAG_BOOTSTRAP="$tag" bash "$SCRIPT_DIR/check-tag-cuda-route.sh" "$tag"
  exit 0
fi

# Post-tag mode: preserve selected_repository_ids across runner-group PATCH.
workflow_ref="fathomadb/fathomdb/.github/workflows/release.yml@refs/tags/${tag}"
group_json="$(gh api orgs/fathomadb/actions/runner-groups/3)"
repos_json="$(gh api orgs/fathomadb/actions/runner-groups/3/repositories)"

repo_ids="$(jq -c '[.repositories[].id]' <<<"$repos_json")"
if [ "$repo_ids" = "[]" ]; then
  printf 'configure-tag-cuda-route: runner group 3 has no selected repositories; refusing to PATCH empty set\n' >&2
  exit 1
fi
if ! jq -e '.repositories | any(.full_name == "fathomadb/fathomdb")' <<<"$repos_json" >/dev/null; then
  printf 'configure-tag-cuda-route: fathomadb/fathomdb is not in runner group 3 selected repositories\n' >&2
  exit 1
fi

# GitHub's PATCH /orgs/{org}/actions/runner-groups/{id} rejects
# selected_repository_ids (HTTP 422) yet may clear selected repositories when
# visibility=selected; restore selected_repository_ids via PUT .../repositories.
patch_body="$(jq -c --arg wf "$workflow_ref" '{
  name: .name,
  visibility: .visibility,
  allows_public_repositories: .allows_public_repositories,
  restricted_to_workflows: .restricted_to_workflows,
  selected_workflows: ((.selected_workflows + [$wf]) | unique)
}' <<<"$group_json")"

gh api -X PATCH orgs/fathomadb/actions/runner-groups/3 --input - <<<"$patch_body" >/dev/null

post_repos_json="$(gh api orgs/fathomadb/actions/runner-groups/3/repositories)"
if ! jq -e '.repositories | any(.full_name == "fathomadb/fathomdb")' <<<"$post_repos_json" >/dev/null; then
  if ! gh api -X PUT orgs/fathomadb/actions/runner-groups/3/repositories --input - <<<"{\"selected_repository_ids\":${repo_ids}}" >/dev/null; then
    printf 'configure-tag-cuda-route: PATCH cleared runner group 3 repositories and PUT restore failed; restore selected_repository_ids=%s manually\n' "$repo_ids" >&2
    exit 1
  fi
  post_repos_json="$(gh api orgs/fathomadb/actions/runner-groups/3/repositories)"
fi

# Verify repository attachment survived PATCH (+ PUT restore if needed) before running the route check.
if ! jq -e '.repositories | any(.full_name == "fathomadb/fathomdb")' <<<"$post_repos_json" >/dev/null; then
  printf 'configure-tag-cuda-route: post-PATCH verification lost fathomadb/fathomdb from runner group 3\n' >&2
  exit 1
fi

bash "$SCRIPT_DIR/check-tag-cuda-route.sh" "$tag"
