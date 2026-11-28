#!/usr/bin/env bash
# scripts/materialize-release-baseline.sh — materialize the governing release
# baseline ref declared by the active release-state JSON (refs/heads/dev/<ver>
# or refs/heads/release/<ver>) in CI checkouts that only create refs/heads/main.
set -euo pipefail

repo_root="$(pwd)"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo-root)
      [ "$#" -ge 2 ] || { printf 'usage: %s [--repo-root DIR]\n' "$(basename "$0")" >&2; exit 2; }
      repo_root="$2"
      shift 2
      ;;
    *)
      printf 'materialize-release-baseline: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

current="$(cd "$repo_root" && python3 scripts/release-current.py)"
if [ -z "$current" ]; then
  exit 0
fi

IFS=$'\t' read -r release _board state_path <<<"$current"
active_ref="$(python3 - "$repo_root/$state_path" "$release" <<'PY'
import json
from pathlib import Path
import sys

state = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
release = sys.argv[2]
allowed = (f"refs/heads/release/{release}", f"refs/heads/dev/{release}")
active_ref = state.get("active_ref")
if active_ref not in allowed:
    raise SystemExit(
        f"materialize-release-baseline: unexpected active_ref {active_ref!r}; "
        f"expected {allowed[0]} or {allowed[1]}"
    )
print(active_ref)
PY
)"

if ! git -C "$repo_root" rev-parse --verify "${active_ref}^{commit}" >/dev/null 2>&1; then
  branch="${active_ref#refs/heads/}"
  remote="refs/remotes/origin/$branch"
  remote_sha="$(git -C "$repo_root" rev-parse --verify "${remote}^{commit}")"
  git -C "$repo_root" update-ref "$active_ref" "$remote_sha"
fi
