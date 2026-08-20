#!/usr/bin/env bash
# RED/GREEN fixtures for resolve_cuda_candidate_sha (0.8.23 Slice 80.6.5).
#
# scripts/release/cuda-preflight.sh cannot run without git on a remote that
# received the source tree with no .git directory. FATHOMDB_CANDIDATE_SHA
# lets that remote supply the SHA directly; this suite proves the resolution
# is byte-identical to the old git-only behavior when unset, and fails closed
# -- never silently accepted, never silently falling through to git -- on any
# explicitly set but invalid value.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../lib/cuda-candidate-sha.sh
. "$REPO_ROOT/scripts/lib/cuda-candidate-sha.sh"

expected_git_sha="$(git -C "$REPO_ROOT" rev-parse HEAD)"

# --- unset falls back to git rev-parse, byte-identical to today ------------
unset FATHOMDB_CANDIDATE_SHA 2>/dev/null || true
CANDIDATE_SHA=""
resolve_cuda_candidate_sha "$REPO_ROOT"
if [ "$CANDIDATE_SHA" != "$expected_git_sha" ]; then
  printf 'FAIL  unset FATHOMDB_CANDIDATE_SHA must fall back to git rev-parse HEAD\n' >&2
  exit 1
fi
printf 'PASS  unset FATHOMDB_CANDIDATE_SHA falls back to git rev-parse HEAD\n'

# --- a valid 40-hex value is used verbatim, git is never consulted ---------
valid_sha="0123456789abcdef0123456789abcdef01234567"
FATHOMDB_CANDIDATE_SHA="$valid_sha"
CANDIDATE_SHA=""
resolve_cuda_candidate_sha "/nonexistent-repo-root-proving-git-is-unused"
if [ "$CANDIDATE_SHA" != "$valid_sha" ]; then
  printf 'FAIL  a valid FATHOMDB_CANDIDATE_SHA must be used verbatim\n' >&2
  exit 1
fi
unset FATHOMDB_CANDIDATE_SHA
printf 'PASS  a valid 40-hex FATHOMDB_CANDIDATE_SHA is used verbatim without consulting git\n'

expect_reject() {
  local value="$1" description="$2"
  local stderr
  CANDIDATE_SHA="unchanged-sentinel"
  FATHOMDB_CANDIDATE_SHA="$value"
  if stderr="$(resolve_cuda_candidate_sha "$REPO_ROOT" 2>&1 1>/dev/null)"; then
    printf 'FAIL  %s: resolve_cuda_candidate_sha must abort, not succeed\n' "$description" >&2
    exit 1
  fi
  if [ "$CANDIDATE_SHA" != "unchanged-sentinel" ]; then
    printf 'FAIL  %s: rejection must never mutate CANDIDATE_SHA\n' "$description" >&2
    exit 1
  fi
  if [ -z "$stderr" ]; then
    printf 'FAIL  %s: rejection must print a diagnostic\n' "$description" >&2
    exit 1
  fi
  unset FATHOMDB_CANDIDATE_SHA 2>/dev/null || true
  printf 'PASS  %s\n' "$description"
}

expect_reject "" "an empty FATHOMDB_CANDIDATE_SHA aborts instead of falling through to git"
expect_reject "0123456789abcdef0123456789abcdef012345" "a 39-character (too-short) FATHOMDB_CANDIDATE_SHA aborts"
expect_reject "0123456789abcdef0123456789abcdef012345g" "a non-hex FATHOMDB_CANDIDATE_SHA aborts"
expect_reject "0123456789ABCDEF0123456789ABCDEF01234567" "an uppercase-hex FATHOMDB_CANDIDATE_SHA aborts"
expect_reject "0123456789abcdef0123456789abcdef0123456789" "an over-long FATHOMDB_CANDIDATE_SHA aborts"

printf '\nCUDA candidate-SHA resolution tests passed\n'
