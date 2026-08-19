#!/usr/bin/env bash
# Resolve the CUDA release candidate SHA without hardcoding a git dependency
# into every call site.
#
# 0.8.23 Slice 80.6.5 (dev/design/0.8.23-aarch64-tegra.md § 7 "80.6.5", "A
# prerequisite found while building this sub-slice's transfer tooling."):
# `scripts/release/cuda-preflight.sh` runs on a remote x86_64 CUDA host that
# receives the source tree with no `.git` directory, deliberately, so the
# no-git rule is structural. `FATHOMDB_CANDIDATE_SHA` lets that remote supply
# the SHA directly.
#
# This value is stamped into a release-evidence witness as a provenance
# claim, so an explicitly set value is validated as a full 40-character
# lowercase hex SHA and fails closed on anything else -- never silently
# accepted, and never silently falls through to `git`. An unset variable
# falls back to `git -C <repo_root> rev-parse HEAD`, unchanged from before
# this variable existed.

# Usage: resolve_cuda_candidate_sha <repo_root>
#
# On success, sets (does not export) the caller's CANDIDATE_SHA variable. On
# failure -- an explicitly set but invalid FATHOMDB_CANDIDATE_SHA -- prints a
# diagnostic to stderr and returns 1 without touching CANDIDATE_SHA.
resolve_cuda_candidate_sha() {
  local repo_root="$1"
  if [ "${FATHOMDB_CANDIDATE_SHA+set}" = set ]; then
    if ! [[ "$FATHOMDB_CANDIDATE_SHA" =~ ^[0-9a-f]{40}$ ]]; then
      printf 'cuda-candidate-sha: FATHOMDB_CANDIDATE_SHA must be a 40-character lowercase hex SHA, got: %s\n' \
        "$FATHOMDB_CANDIDATE_SHA" >&2
      return 1
    fi
    # shellcheck disable=SC2034  # consumed by the caller (cuda-preflight.sh), not within this library.
    CANDIDATE_SHA="$FATHOMDB_CANDIDATE_SHA"
  else
    # shellcheck disable=SC2034  # consumed by the caller (cuda-preflight.sh), not within this library.
    CANDIDATE_SHA="$(git -C "$repo_root" rev-parse HEAD)"
  fi
}
