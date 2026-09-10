#!/usr/bin/env bash
# Bind final-candidate GitHub Actions evidence to an exact commit.
set -euo pipefail

usage() {
  printf 'usage: %s --repository OWNER/REPO --branch BRANCH --candidate-sha SHA [--dispatch] (--job NAME | --all-required) --receipt FILE\n' "$0" >&2
}

repository='' branch='' candidate_sha='' job='' all_required=false dispatch=false receipt=''
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
    --dispatch) dispatch=true; shift ;;
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
run_id=''
if [ "$dispatch" = true ]; then
  gh run list --repo "$repository" --workflow CI --event workflow_dispatch --limit 100 \
    --json databaseId > "$work/before.json"
  gh workflow run ci.yml --repo "$repository" --ref "$branch" -f "candidate_sha=$candidate_sha"
  for _ in $(seq 1 60); do
    gh run list --repo "$repository" --workflow CI --branch "$branch" \
      --event workflow_dispatch --limit 20 --json databaseId,headSha > "$work/after.json"
    run_id="$(python3 - "$work/before.json" "$work/after.json" "$candidate_sha" <<'PY'
import json, pathlib, sys
before = {row["databaseId"] for row in json.loads(pathlib.Path(sys.argv[1]).read_text())}
after = json.loads(pathlib.Path(sys.argv[2]).read_text())
matches = [row for row in after if row["databaseId"] not in before and row["headSha"] == sys.argv[3]]
print(matches[0]["databaseId"] if len(matches) == 1 else "")
PY
)"
    [ -z "$run_id" ] || break
    sleep 5
  done
  [ -n "$run_id" ] \
    || { printf 'slice75-hosted-ci: dispatched run was not identified\n' >&2; exit 1; }
  gh run watch "$run_id" --repo "$repository" --exit-status
fi
gh run list --repo "$repository" --workflow CI --branch "$branch" --commit "$candidate_sha" \
  --limit 20 --json databaseId,headSha,name,status,conclusion,event,createdAt,url > "$work/runs.json"
python3 - "$work/runs.json" "$repository" "$branch" "$candidate_sha" "$job" \
  "$all_required" "$run_id" "$receipt" <<'PY'
import json
import pathlib
import subprocess
import sys

runs_path, repository, branch, candidate_sha, selected_job, all_required, requested_run, receipt_path = sys.argv[1:]
runs = json.loads(pathlib.Path(runs_path).read_text())
eligible = [
    run for run in runs
    if run.get("headSha") == candidate_sha
    and (not requested_run or str(run.get("databaseId")) == requested_run)
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
    bad = [value for value in selected if value.get("status") != "completed" or value.get("conclusion") != "success"]
    if bad:
        raise SystemExit("slice75-hosted-ci: selected jobs did not pass: " + ", ".join(v["name"] for v in bad))
    payload.update({
        "mode": "selected_job",
        "job": selected_job,
        "rows": expected_rows,
        "jobs": [{"name": value["name"], "conclusion": value["conclusion"]} for value in selected],
    })
elif all_required == "true":
    def belongs(name, job_id):
        return name == job_id or name.startswith(job_id + " (")

    required = [
        "shell-lint", "changes", "verify-fast", "verify", "security",
        "default-embedder-tests", "windows-wal-checkpoint-diagnosis",
        "windows-wal-attribution", "wheel-size-gate",
        "native-artifact-runtime-validation", "board-currency",
        "ledger-integrity", "plan-anchors", "governed-surface-pin",
        "pinned-override-rot", "c1-contract-conformance",
        "transcript-hygiene", "release-state-views", "commission-manifest",
        "design-status", "steward-orient", "docs",
    ]
    missing = [name for name in required if not any(belongs(value["name"], name) for value in jobs)]
    failed = [
        value["name"] for value in jobs
        if value.get("status") != "completed"
        or value.get("conclusion") not in {"success", "skipped"}
    ]
    skipped = [value["name"] for value in jobs if value.get("conclusion") == "skipped"]
    unauthorized_skips = [name for name in skipped if name != "markdownlint"]
    required_bad = [
        value["name"] for value in jobs
        if any(belongs(value["name"], name) for name in required)
        and value.get("conclusion") != "success"
    ]
    if missing or failed or unauthorized_skips or required_bad:
        raise SystemExit(
            "slice75-hosted-ci: required inventory failed: "
            f"missing={missing}, failed={failed}, unauthorized_skips={unauthorized_skips}, "
            f"required_bad={required_bad}"
        )
    advisory = [
        {"name": value["name"], "conclusion": value["conclusion"]}
        for value in jobs
        if value["name"].startswith(("gitleaks", "rust-workspace-race-report"))
    ]
    payload.update({
        "mode": "all_required",
        "required_conclusion": "success",
        "required_jobs": required,
        "unauthorized_skips": len(unauthorized_skips),
        "advisory_jobs_recorded": True,
        "advisory_jobs": advisory,
        "jobs": [{"name": value["name"], "conclusion": value["conclusion"]} for value in jobs],
    })
path = pathlib.Path(receipt_path)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
printf 'slice75-hosted-ci: pass; receipt at %s\n' "$receipt"
