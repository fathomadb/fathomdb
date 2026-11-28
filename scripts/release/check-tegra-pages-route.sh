#!/usr/bin/env bash
# Fail before a Tegra build if its release branch cannot deploy to Pages.
set -euo pipefail

version="${1:-}"
repository="${2:-fathomadb/fathomdb}"
candidate_sha="${3:-}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'check-tegra-pages-route: expected X.Y.Z release version\n' >&2
  exit 2
fi
if ! [[ "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  printf 'check-tegra-pages-route: expected OWNER/REPO\n' >&2
  exit 2
fi
if [ -n "$candidate_sha" ] && ! [[ "$candidate_sha" =~ ^[0-9a-f]{40}$ ]]; then
  printf 'check-tegra-pages-route: optional candidate_sha must be 40 lowercase hex characters\n' >&2
  exit 2
fi
command -v gh >/dev/null || {
  printf 'check-tegra-pages-route: gh is required to inspect Pages routing\n' >&2
  exit 1
}
command -v jq >/dev/null || {
  printf 'check-tegra-pages-route: jq is required to inspect Pages routing\n' >&2
  exit 1
}

env_json="$(gh api "repos/$repository/environments/github-pages/deployment-branch-policies")" || {
  printf 'check-tegra-pages-route: cannot read github-pages deployment policies\n' >&2
  exit 1
}
branch="release/$version"
if ! jq -e --arg branch "$branch" '
  .branch_policies | any(.type == "branch" and .name == $branch)
' <<<"$env_json" >/dev/null; then
  printf 'check-tegra-pages-route: github-pages does not allow branch %s\n' "$branch" >&2
  exit 1
fi
if [ -n "$candidate_sha" ]; then
  deployments_json="$(gh api "repos/$repository/deployments?environment=github-pages&per_page=10")" || {
    printf 'check-tegra-pages-route: cannot read github-pages deployments\n' >&2
    exit 1
  }
  if jq -e --arg sha "$candidate_sha" 'any(.[]; .sha == $sha)' <<<"$deployments_json" >/dev/null; then
    printf 'check-tegra-pages-route: %s is already deployed to github-pages; GitHub Pages deduplicates actions/deploy-pages by pages_build_version (GITHUB_SHA). Advance main and %s to a distinct commit before dispatching Tegra Pages publication.\n' \
      "$candidate_sha" "$branch" >&2
    exit 1
  fi
fi
printf 'check-tegra-pages-route: %s can deploy to github-pages\n' "$branch"
