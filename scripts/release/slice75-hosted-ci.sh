#!/usr/bin/env bash
# Bind final-candidate GitHub Actions evidence to an exact commit.
set -euo pipefail

usage() {
  printf 'usage: %s --repository OWNER/REPO --branch BRANCH --candidate-sha SHA (--job NAME | --all-required) --receipt FILE\n' "$0" >&2
}

repository='' branch='' candidate_sha='' job='' all_required=false receipt=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --repository|--branch|--candidate-sha|--job|--receipt)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      case "$1" in
        --repository) repository="$2" ;;
        --branch) branch="$2" ;;
        --candidate-sha) candidate_sha="$2" ;;
        --job) job="$2" ;;
        --receipt) receipt="$2" ;;
      esac
      shift 2
      ;;
    --all-required) all_required=true; shift ;;
    *) usage; exit 2 ;;
  esac
done
[ -n "$repository" ] && [ -n "$branch" ] && [ -n "$candidate_sha" ] && [ -n "$receipt" ] \
  || { usage; exit 2; }
[[ "$candidate_sha" =~ ^[0-9a-f]{40}$ ]] || { printf 'slice75-hosted-ci: invalid candidate SHA\n' >&2; exit 2; }
if { [ -n "$job" ] && [ "$all_required" = true ]; } || { [ -z "$job" ] && [ "$all_required" = false ]; }; then
  usage
  exit 2
fi
command -v gh >/dev/null || { printf 'slice75-hosted-ci: gh is required\n' >&2; exit 1; }
gh auth status >/dev/null 2>&1 || { printf 'slice75-hosted-ci: gh authentication is unavailable\n' >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
gh run list --repo "$repository" --workflow CI --branch "$branch" --commit "$candidate_sha" \
  --limit 20 --json databaseId,headSha,name,status,conclusion,event,createdAt,url > "$work/runs.json"
python3 - "$work/runs.json" "$repository" "$branch" "$candidate_sha" "$job" \
  "$all_required" "$receipt" <<'PY'
import json
import pathlib
import subprocess
import sys

runs_path, repository, branch, candidate_sha, selected_job, all_required, receipt_path = sys.argv[1:]
runs = json.loads(pathlib.Path(runs_path).read_text())
eligible = [
    run for run in runs
    if run.get("headSha") == candidate_sha
    and run.get("status") == "completed"
    and run.get("conclusion") == "success"
]
if not eligible:
    raise SystemExit("slice75-hosted-ci: no successful completed CI run at the exact candidate SHA")
run = max(eligible, key=lambda value: value["createdAt"])
view = subprocess.run(
    ["gh", "run", "view", str(run["databaseId"]), "--repo", repository, "--json", "jobs"],
    check=True,
    capture_output=True,
    text=True,
)
jobs = json.loads(view.stdout)["jobs"]
bad = [value for value in jobs if value.get("status") != "completed" or value.get("conclusion") != "success"]
if bad:
    raise SystemExit("slice75-hosted-ci: exact-head run contains unsuccessful jobs: " + ", ".join(v["name"] for v in bad))

payload = {
    "schema_version": "fathomdb.slice75-hosted-ci/v1",
    "repository": repository,
    "branch": branch,
    "candidate_sha": candidate_sha,
    "run_id": run["databaseId"],
    "run_url": run["url"],
    "conclusion": "success",
}
if selected_job:
    expected_rows = ["linux-x64-gnu", "linux-arm64-gnu", "darwin-x64", "darwin-arm64", "win32-x64-msvc"]
    selected = [value for value in jobs if selected_job in value["name"]]
    missing = [row for row in expected_rows if not any(row in value["name"] for value in selected)]
    if missing or len(selected) != len(expected_rows):
        raise SystemExit(f"slice75-hosted-ci: {selected_job} rows differ; missing={missing}, count={len(selected)}")
    payload.update({
        "mode": "selected_job",
        "job": selected_job,
        "rows": expected_rows,
        "jobs": [{"name": value["name"], "conclusion": value["conclusion"]} for value in selected],
    })
elif all_required == "true":
    payload.update({
        "mode": "all_required",
        "required_conclusion": "success",
        "unauthorized_skips": 0,
        "advisory_jobs_recorded": True,
        "jobs": [{"name": value["name"], "conclusion": value["conclusion"]} for value in jobs],
    })
path = pathlib.Path(receipt_path)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
printf 'slice75-hosted-ci: pass; receipt at %s\n' "$receipt"
