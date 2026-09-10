#!/usr/bin/env bash
# Dispatch and retain the exact-head, nonpublishing Jetson/Tegra evidence run.
set -euo pipefail

usage() {
  printf 'usage: %s --repository OWNER/REPO --workflow FILE --branch BRANCH --candidate-sha SHA --candidate-version VERSION --receipt FILE\n' "$0" >&2
}

repository='' workflow='' branch='' candidate_sha='' candidate_version='' receipt=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --repository) repository="$2" ;;
    --workflow) workflow="$2" ;;
    --branch) branch="$2" ;;
    --candidate-sha) candidate_sha="$2" ;;
    --candidate-version) candidate_version="$2" ;;
    --receipt) receipt="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done
[ -n "$repository" ] && [ -n "$workflow" ] && [ -n "$branch" ] \
  && [ -n "$candidate_sha" ] && [ -n "$candidate_version" ] && [ -n "$receipt" ] \
  || { usage; exit 2; }
[[ "$candidate_sha" =~ ^[0-9a-f]{40}$ ]] || { printf 'slice75-jetson-run: invalid SHA\n' >&2; exit 2; }
[[ "$candidate_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
  || { printf 'slice75-jetson-run: invalid version\n' >&2; exit 2; }
[ "$branch" = "release/$candidate_version" ] \
  || { printf 'slice75-jetson-run: branch and candidate version differ\n' >&2; exit 2; }
gh auth status >/dev/null 2>&1 || { printf 'slice75-jetson-run: gh authentication is unavailable\n' >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
gh run list --repo "$repository" --workflow "$workflow" --event workflow_dispatch --limit 100 \
  --json databaseId > "$work/before.json"
gh workflow run "$workflow" --repo "$repository" --ref "$branch" \
  -f "candidate_sha=$candidate_sha" -f "candidate_version=$candidate_version" \
  -f publish_to_pages=false

run_id=''
for _ in $(seq 1 60); do
  gh run list --repo "$repository" --workflow "$workflow" --branch "$branch" \
    --event workflow_dispatch --limit 20 --json databaseId,headSha,status,conclusion,url > "$work/after.json"
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
[ -n "$run_id" ] || { printf 'slice75-jetson-run: dispatched run was not identified\n' >&2; exit 1; }
gh run watch "$run_id" --repo "$repository" --exit-status
gh run view "$run_id" --repo "$repository" --json headSha,status,conclusion,url,jobs > "$work/run.json"
artifact_name="jetson-tegra-cuda-evidence-$run_id-1"
mkdir "$work/evidence"
gh run download "$run_id" --repo "$repository" --name "$artifact_name" --dir "$work/evidence"
python3 scripts/release/verify-tegra-gpu-witness.py \
  --witness "$work/evidence/tegra-gpu-allocation-witness.json"
python3 - "$work/run.json" "$work/evidence" "$repository" "$workflow" "$branch" \
  "$candidate_sha" "$candidate_version" "$run_id" "$receipt" <<'PY'
import hashlib
import json
import pathlib
import sys

run_path, evidence_text, repository, workflow, branch, sha, version, run_id, receipt_text = sys.argv[1:]
run = json.loads(pathlib.Path(run_path).read_text())
if run["headSha"] != sha or run["status"] != "completed" or run["conclusion"] != "success":
    raise SystemExit("slice75-jetson-run: run did not pass at the requested SHA")
jobs = run["jobs"]
published = [job for job in jobs if "Deploy" in job["name"] and job["conclusion"] != "skipped"]
if published:
    raise SystemExit("slice75-jetson-run: a publication job executed")
evidence = pathlib.Path(evidence_text)
wheels = list((evidence / "wheel").glob(f"fathomdb-{version}+tegra-*-linux_aarch64.whl"))
if len(wheels) != 1:
    raise SystemExit(f"slice75-jetson-run: expected one {version}+tegra wheel, found {len(wheels)}")
witness = json.loads((evidence / "tegra-gpu-allocation-witness.json").read_text())
source = (evidence / "source-identity.txt").read_text()
if f"candidate_sha={sha}" not in source or f"checkout_sha={sha}" not in source:
    raise SystemExit("slice75-jetson-run: downloaded source identity differs")
payload = {
    "schema_version": "fathomdb.slice75-jetson/v1",
    "repository": repository,
    "workflow": workflow,
    "branch": branch,
    "candidate_sha": sha,
    "candidate_version": version,
    "run_id": int(run_id),
    "run_url": run["url"],
    "runner_host": "10.83.10.13",
    "wheel_count": 1,
    "wheel_filename": wheels[0].name,
    "wheel_sha256": hashlib.sha256(wheels[0].read_bytes()).hexdigest(),
    "gpu": witness.get("device_name"),
    "policies": ["cpu", "auto", "cuda"],
    "publication_jobs": 0,
    "outcome": "pass",
}
path = pathlib.Path(receipt_text)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
printf 'slice75-jetson-run: pass; run %s; receipt at %s\n' "$run_id" "$receipt"
