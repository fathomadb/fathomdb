#!/usr/bin/env bash
# TDD unit tests for scripts/materialize-release-baseline.sh.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/materialize-release-baseline.sh"
PREFLIGHT_STATE="$REPO_ROOT/scripts/preflight-release-state.py"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

make_fixture() {
  local dir="$1"
  local active_ref="$2"
  local create_remote="$3"
  mkdir -p "$dir"
  git -C "$dir" init -q -b main
  git -C "$dir" config user.email "retro@example.invalid"
  git -C "$dir" config user.name "Retro"
  git -C "$dir" config commit.gpgsign false
  mkdir -p "$dir/dev/plans/runs" "$dir/dev/plans/9.2.0" "$dir/scripts"
  cp "$REPO_ROOT/scripts/release-current.py" "$dir/scripts/release-current.py"
  printf '# board\n' > "$dir/dev/plans/runs/STATUS-9.2.0.md"
  printf '# plan\n' > "$dir/dev/plans/9.2.0/plan.md"
  printf 'seed\n' > "$dir/seed"
  git -C "$dir" add seed
  git -C "$dir" commit -qm "seed"
  local dep_sha
  dep_sha="$(git -C "$dir" rev-parse HEAD)"
  cat > "$dir/dev/plans/release-state-9.2.0.json" <<EOF
{
  "release": "9.2.0",
  "board": "dev/plans/runs/STATUS-9.2.0.md",
  "plan": "dev/plans/9.2.0/plan.md",
  "active_ref": "$active_ref",
  "published": null,
  "completion": null,
  "ladder": [
    {
      "slice": 60,
      "status": "LANDED",
      "sha": "$dep_sha"
    }
  ]
}
EOF
  git -C "$dir" add dev scripts
  git -C "$dir" commit -qm "release state"
  local head_sha
  head_sha="$(git -C "$dir" rev-parse HEAD)"
  git -C "$dir" update-ref refs/remotes/origin/main "$head_sha"
  if [ "$create_remote" -eq 1 ]; then
    local branch_name="${active_ref#refs/heads/}"
    git -C "$dir" update-ref "refs/remotes/origin/$branch_name" "$head_sha"
  fi
}

# Arm 1: dev/9.2.0 active_ref materialized from refs/remotes/origin/dev/9.2.0 on main checkout.
make_fixture "$WORK/dev-case" "refs/heads/dev/9.2.0" 1
bash "$SCRIPT" --repo-root "$WORK/dev-case"
git -C "$WORK/dev-case" rev-parse --verify "refs/heads/dev/9.2.0^{commit}" >/dev/null
dev_head="$(git -C "$WORK/dev-case" rev-parse HEAD)"
python3 "$PREFLIGHT_STATE" --repo-root "$WORK/dev-case" \
  --release 9.2.0 --board dev/plans/runs/STATUS-9.2.0.md \
  --state dev/plans/release-state-9.2.0.json --plan dev/plans/9.2.0/plan.md \
  --expect-closed 60 --target-head "$dev_head" >/dev/null
printf 'PASS arm1 materializes refs/heads/dev/9.2.0 on main CI checkout\n'

# Arm 2: release/9.2.0 active_ref materialized from refs/remotes/origin/release/9.2.0.
make_fixture "$WORK/rel-case" "refs/heads/release/9.2.0" 1
bash "$SCRIPT" --repo-root "$WORK/rel-case"
git -C "$WORK/rel-case" rev-parse --verify "refs/heads/release/9.2.0^{commit}" >/dev/null
rel_head="$(git -C "$WORK/rel-case" rev-parse HEAD)"
python3 "$PREFLIGHT_STATE" --repo-root "$WORK/rel-case" \
  --release 9.2.0 --board dev/plans/runs/STATUS-9.2.0.md \
  --state dev/plans/release-state-9.2.0.json --plan dev/plans/9.2.0/plan.md \
  --expect-closed 60 --target-head "$rel_head" >/dev/null
printf 'PASS arm2 materializes refs/heads/release/9.2.0 on main CI checkout\n'

# Arm 3: missing remote tracking ref fails closed.
make_fixture "$WORK/missing-remote" "refs/heads/dev/9.2.0" 0
rc=0
bash "$SCRIPT" --repo-root "$WORK/missing-remote" >/dev/null 2>&1 || rc=$?
[ "$rc" -ne 0 ] || { printf 'FAIL [arm3]: expected nonzero exit when remote ref missing\n' >&2; exit 1; }
printf 'PASS arm3 fails closed when origin/dev/9.2.0 is absent\n'

# Arm 4: no active release (published state only) is a clean no-op.
mkdir -p "$WORK/published-only/dev/plans/runs" "$WORK/published-only/scripts"
git -C "$WORK/published-only" init -q -b main
git -C "$WORK/published-only" config user.email "retro@example.invalid"
git -C "$WORK/published-only" config user.name "Retro"
git -C "$WORK/published-only" config commit.gpgsign false
cp "$REPO_ROOT/scripts/release-current.py" "$WORK/published-only/scripts/release-current.py"
printf '# closed\n' > "$WORK/published-only/dev/plans/runs/STATUS-9.2.0.md"
cat > "$WORK/published-only/dev/plans/release-state-9.2.0.json" <<'EOF'
{
  "release": "9.2.0",
  "board": "dev/plans/runs/STATUS-9.2.0.md",
  "published": {
    "tag": "v9.2.0",
    "tag_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "published_on": "2026-11-28",
    "npm_dist_tag": "latest"
  }
}
EOF
git -C "$WORK/published-only" add dev scripts
git -C "$WORK/published-only" commit -qm "published"
bash "$SCRIPT" --repo-root "$WORK/published-only"
printf 'PASS arm4 clean no-op when no active unpublished release exists\n'
