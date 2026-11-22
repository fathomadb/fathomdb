#!/usr/bin/env bash
# Dynamic Cargo.toml version derivation tests for scripts/release/check-tag-cuda-route.sh (P7-10).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
GUARD="$REPO_ROOT/scripts/release/check-tag-cuda-route.sh"

FIXTURE="$(mktemp -d)"
trap 'rm -rf "$FIXTURE"' EXIT

cat >"$FIXTURE/gh" <<'SHIM'
#!/usr/bin/env bash
case "$2" in
  orgs/fathomadb/actions/runner-groups/3)
    printf '{"name":"fathomdb-gpu-release","visibility":"selected","restricted_to_workflows":true,"selected_workflows":["fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main"]}\n'
    ;;
  repos/fathomadb/fathomdb/environments/cuda-unmerged-preflight/deployment-branch-policies)
    printf '{"branch_policies":[{"name":"main","type":"branch"},{"name":"%s","type":"tag"}]}\n' "$TEST_ALLOWED_TAG"
    ;;
  repos/fathomadb/fathomdb/environments/cuda-unmerged-preflight)
    printf '{"protection_rules":[{"type":"required_reviewers","reviewers":[{"type":"User","reviewer":{"id":123}}]}]}\n'
    ;;
  repos/fathomadb/fathomdb/git/ref/tags/*)
    printf 'HTTP/2.0 404 Not Found\r\nContent-Type: application/json\r\n\r\n{"message":"Not Found","status":"404"}\n'
    exit 1
    ;;
  *) exit 2 ;;
esac
SHIM
chmod +x "$FIXTURE/gh"

cat >"$FIXTURE/Cargo.toml" <<'TOML'
[workspace]
members = []

[workspace.package]
version = "0.8.31"
TOML

# 1. Dynamic version from overridden Cargo.toml accepts v0.8.31 without hardcoding.
out="$(PATH="$FIXTURE:$PATH" \
  FATHOMDB_CARGO_TOML="$FIXTURE/Cargo.toml" \
  FATHOMDB_RELEASE_TAG_BOOTSTRAP=v0.8.31 \
  TEST_ALLOWED_TAG=v0.8.31 \
  bash "$GUARD" v0.8.31)"
[[ "$out" == *"bootstrap ready for v0.8.31"* ]]
printf 'PASS dynamic Cargo.toml version v0.8.31 accepted for bootstrap\n'

# 2. Tag mismatching Cargo.toml version is rejected even if environment allows it.
if PATH="$FIXTURE:$PATH" \
  FATHOMDB_CARGO_TOML="$FIXTURE/Cargo.toml" \
  FATHOMDB_RELEASE_TAG_BOOTSTRAP=v0.8.32 \
  TEST_ALLOWED_TAG=v0.8.32 \
  bash "$GUARD" v0.8.32 >/dev/null 2>&1; then
  printf 'FAIL mismatched tag v0.8.32 was accepted against Cargo.toml 0.8.31\n' >&2
  exit 1
fi
printf 'PASS tag mismatching Cargo.toml version rejected\n'

# 3. Bootstrap marker mismatching requested tag is rejected.
if PATH="$FIXTURE:$PATH" \
  FATHOMDB_CARGO_TOML="$FIXTURE/Cargo.toml" \
  FATHOMDB_RELEASE_TAG_BOOTSTRAP=v0.8.30 \
  TEST_ALLOWED_TAG=v0.8.31 \
  bash "$GUARD" v0.8.31 >/dev/null 2>&1; then
  printf 'FAIL mismatched bootstrap marker was accepted\n' >&2
  exit 1
fi
printf 'PASS mismatched bootstrap marker rejected\n'
