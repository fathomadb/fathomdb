#!/usr/bin/env bash
# Dispatch, verify, and smoke the exact-version Tegra Pages publication route.
# One Jetson dispatch publishes the `+tegra` wheel and the Jetson Node tarball
# together; publication is complete only after both post-publication smokes
# pass from the public Pages bytes. Nothing is published to a registry.
set -euo pipefail

usage() {
  printf 'usage: %s --candidate-version X.Y.Z --candidate-sha 40-hex [--repository OWNER/REPO] [--skip-pages-smoke]\n' "$(basename "$0")" >&2
}

repository='fathomadb/fathomdb'
candidate_version=''
candidate_sha=''
skip_pages_smoke=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --repository) repository="${2:-}"; shift 2 ;;
    --candidate-version) candidate_version="${2:-}"; shift 2 ;;
    --candidate-sha) candidate_sha="${2:-}"; shift 2 ;;
    --skip-pages-smoke) skip_pages_smoke=1; shift ;;
    *) usage; exit 2 ;;
  esac
done

[[ "$candidate_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  printf 'publish-tegra-pages: candidate version must be X.Y.Z\n' >&2; exit 2;
}
[[ "$candidate_sha" =~ ^[0-9a-f]{40}$ ]] || {
  printf 'publish-tegra-pages: candidate SHA must be 40 lowercase hexadecimal characters\n' >&2; exit 2;
}
command -v gh >/dev/null 2>&1 || { printf 'publish-tegra-pages: gh is required\n' >&2; exit 1; }
gh auth status >/dev/null 2>&1 || { printf 'publish-tegra-pages: gh authentication is unavailable\n' >&2; exit 1; }

branch="release/$candidate_version"
remote_head="$(git ls-remote "https://github.com/$repository.git" "refs/heads/$branch" | awk 'NR == 1 { print $1 }')"
[ "$remote_head" = "$candidate_sha" ] || {
  printf 'publish-tegra-pages: remote %s is %s, not requested %s\n' "$branch" "${remote_head:-missing}" "$candidate_sha" >&2
  exit 1
}

work="$(mktemp -d)"
cleanup() { rm -rf "$work"; }
trap cleanup EXIT
workflow='Jetson Tegra CUDA evidence'
gh run list --repo "$repository" --workflow "$workflow" --limit 100 \
  --json databaseId > "$work/before.json"
gh workflow run "$workflow" --repo "$repository" --ref "$branch" \
  -f "candidate_sha=$candidate_sha" -f "candidate_version=$candidate_version" -f publish_to_pages=true

run_id=''
for _ in $(seq 1 60); do
  gh run list --repo "$repository" --workflow "$workflow" --limit 20 \
    --json databaseId,headBranch,headSha,status,conclusion,url > "$work/after.json"
  run_id="$(python3 - "$work/before.json" "$work/after.json" "$branch" "$candidate_sha" <<'PY'
import json
from pathlib import Path
import sys

before = {row["databaseId"] for row in json.loads(Path(sys.argv[1]).read_text())}
rows = json.loads(Path(sys.argv[2]).read_text())
matches = [
    row for row in rows
    if row["databaseId"] not in before
    and row["headBranch"] == sys.argv[3]
    and row["headSha"] == sys.argv[4]
]
print(matches[0]["databaseId"] if len(matches) == 1 else "")
PY
)"
  [ -n "$run_id" ] && break
  sleep 5
done
[ -n "$run_id" ] || { printf 'publish-tegra-pages: dispatched run was not identified\n' >&2; exit 1; }
gh run watch "$run_id" --repo "$repository" --exit-status
gh run view "$run_id" --repo "$repository" --json headSha,status,conclusion,url > "$work/run.json"
python3 - "$work/run.json" "$candidate_sha" <<'PY'
import json
from pathlib import Path
import sys

run = json.loads(Path(sys.argv[1]).read_text())
if run["headSha"] != sys.argv[2] or run["status"] != "completed" or run["conclusion"] != "success":
    raise SystemExit("publish-tegra-pages: CI did not succeed at the requested SHA")
print(run["url"])
PY

if [ "$skip_pages_smoke" -eq 1 ]; then
  printf 'publish-tegra-pages: INCOMPLETE: Pages smoke skipped by explicit request\n' >&2
  exit 3
fi
gh run download "$run_id" --repo "$repository" \
  --name "jetson-tegra-cuda-evidence-$run_id-1" --dir "$work/evidence"
wheel_sha256="$(awk 'NR == 1 { print $1 }' "$work/evidence/wheel.sha256")"
[[ "$wheel_sha256" =~ ^[0-9a-f]{64}$ ]] || {
  printf 'publish-tegra-pages: retained evidence has no valid wheel SHA-256\n' >&2; exit 1;
}
wheel_name="$(awk 'NR == 1 { print $2 }' "$work/evidence/wheel.sha256")"
wheel_name="${wheel_name##*/}"
node_name="fathomdb-linux-arm64-gnu-tegra-$candidate_version.tgz"
node_sha256=''
if [ -f "$work/evidence/node/SHA256SUMS" ]; then
  node_sha256="$(awk -v name="$node_name" '$2 == name { print $1 }' "$work/evidence/node/SHA256SUMS")"
fi
[[ "$node_sha256" =~ ^[0-9a-f]{64}$ ]] || {
  printf 'publish-tegra-pages: retained evidence has no valid Node tarball SHA-256 for %s\n' "$node_name" >&2; exit 1;
}
pages_packages='https://fathomadb.github.io/fathomdb/tegra/packages'
evidence_dir="$(pwd)/tegra-pages-smoke-$candidate_version-$run_id"
node_evidence_dir="$(pwd)/tegra-pages-node-smoke-$candidate_version-$run_id"
for path in "$evidence_dir" "$node_evidence_dir"; do
  [ ! -e "$path" ] || { printf 'publish-tegra-pages: evidence path already exists: %s\n' "$path" >&2; exit 1; }
done
if ! bash scripts/release/smoke/smoke-tegra-pages-wheel.sh \
  "$candidate_version+tegra" https://fathomadb.github.io/fathomdb/tegra/simple/ \
  "$wheel_sha256" "$candidate_sha" "$run_id" "$evidence_dir"; then
  printf 'publish-tegra-pages: INCOMPLETE: the post-publication wheel smoke failed; see %s\n' "$evidence_dir" >&2
  exit 1
fi
if ! bash scripts/release/smoke/smoke-tegra-node-package.sh --source pages \
  --version "$candidate_version" --pages-url "$pages_packages/$node_name" \
  --tegra-sha256 "$node_sha256" --pages-run "$run_id" \
  --candidate-sha "$candidate_sha" --evidence-dir "$node_evidence_dir"; then
  printf 'publish-tegra-pages: INCOMPLETE: the post-publication Node smoke failed; see %s\n' "$node_evidence_dir" >&2
  exit 1
fi
printf 'publish-tegra-pages: published and smoke-tested fathomdb==%s+tegra and %s\n' \
  "$candidate_version" "$node_name"
# Every later Pages deployment rebuilds only what the retained list names, so
# the published pair must be added to it before the next deployment.
printf 'publish-tegra-pages: add these entries to scripts/release/tegra-pages-retained.json:\n'
python3 - "$candidate_version" "$wheel_name" "$wheel_sha256" "$node_name" "$node_sha256" "$pages_packages" <<'PY'
import json
import sys

version, wheel, wheel_sha256, tarball, tarball_sha256, base = sys.argv[1:]
entries = [
    {"kind": "wheel", "version": f"{version}+tegra", "filename": wheel, "sha256": wheel_sha256, "url": f"{base}/{wheel}"},
    {"kind": "node-tarball", "version": version, "filename": tarball, "sha256": tarball_sha256, "url": f"{base}/{tarball}"},
]
print(json.dumps(entries, indent=2))
PY
