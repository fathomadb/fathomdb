#!/usr/bin/env python3
"""Verify original production evidence before maintained npm recovery."""

import argparse
import json
import os
import re
import subprocess

VERSION = "0.8.30"
RUN_ID = "44929252460"
SOURCE = "e3d31f5cdc0df3e32dab9bad151c5ab5c321222c"
TAG_OBJECT = "fbae0d3319fb31bb62360977fded676f2dea8557"
GOOD = (
    "publish-rust-t7-cli", "publish-pypi", "publish-npm",
    "post-publish-smoke (crates-cli)", "post-publish-smoke (crates-sdk)",
    "post-publish-smoke (npm-package)", "post-publish-smoke-aarch64",
    "post-publish-smoke-darwin-x64", "post-publish-smoke-win32-x64",
    "co-tagging-assert",
)
AFFECTED = ("post-publish-smoke (pypi-wheel)", "post-publish-smoke-darwin-arm64")

VERSION_0831 = "0.8.31"
RUN_ID_0831 = "45318429916"
SOURCE_0831 = "8965af93d6f42d160474acb382596c5aab6b5c2d"
TAG_OBJECT_0831 = "48a224e5ac691d5b88a7979b920e87352d31e87f"
GOOD_0831 = (
    "publish-rust-t7-cli", "publish-pypi", "publish-npm",
    "post-publish-smoke (crates-cli)", "post-publish-smoke (pypi-wheel)",
    "post-publish-smoke (npm-package)", "post-publish-smoke-aarch64",
    "post-publish-smoke-darwin-x64", "post-publish-smoke-darwin-arm64",
    "post-publish-smoke-win32-x64", "co-tagging-assert",
)
AFFECTED_0831 = ("post-publish-smoke (crates-sdk)",)


def one_named(rows, name):
    matches = [row for row in rows if row.get("name") == name]
    if len(matches) != 1:
        raise ValueError(f"expected one original proof: {name}")
    return matches[0]


def completed(row, conclusion):
    if row.get("status") != "completed" or row.get("conclusion") != conclusion:
        raise ValueError(f"original proof must be completed/{conclusion}: {row.get('name')}")


def validate(*, version, run_id, candidate_sha, recover, tag_object, tag_commit, run, jobs, repository,
             recover_0831_crates_sdk=False):
    """Refuse wrong identity or missing original proofs; return only on acceptance."""
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("expected exact stable version")
    if not re.fullmatch(r"[0-9]+", run_id) or not re.fullmatch(r"[0-9a-f]{40}", candidate_sha):
        raise ValueError("invalid run/source")
    if recover and recover_0831_crates_sdk:
        raise ValueError("choose at most one release-specific smoke recovery mode")
    if tag_commit != candidate_sha:
        raise ValueError("tag source mismatch")
    expected = {
        "id": int(run_id), "event": "push", "head_branch": f"v{version}",
        "head_sha": candidate_sha, "path": ".github/workflows/release.yml", "status": "completed",
    }
    if any(run.get(key) != value for key, value in expected.items()):
        raise ValueError("not the completed production tag run")
    if run.get("repository", {}).get("full_name") != repository or repository != "fathomadb/fathomdb":
        raise ValueError("production repository mismatch")
    if version == VERSION and not recover:
        raise ValueError("0.8.30 requires corrected affected-smoke recovery; tag smokes are unqualified")
    if recover_0831_crates_sdk:
        if (version, run_id, candidate_sha, tag_object) != (
            VERSION_0831, RUN_ID_0831, SOURCE_0831, TAG_OBJECT_0831
        ):
            raise ValueError("0.8.31 crates-sdk recovery is bound to the original 0.8.31 release")
        if run.get("run_attempt") != 1 or run.get("conclusion") != "failure":
            raise ValueError("expected original failed production attempt 1")
        for name in GOOD_0831:
            completed(one_named(jobs, name), "success")
        for name in AFFECTED_0831:
            completed(one_named(jobs, name), "failure")
        return
    if recover:
        if (version, run_id, candidate_sha, tag_object) != (VERSION, RUN_ID, SOURCE, TAG_OBJECT):
            raise ValueError("affected-smoke recovery is bound to the original 0.8.30 release")
        if run.get("run_attempt") != 1 or run.get("conclusion") != "failure":
            raise ValueError("expected original failed production attempt 1")
    for name in GOOD:
        completed(one_named(jobs, name), "success")
    for name in AFFECTED:
        completed(one_named(jobs, name), "failure" if recover else "success")
    if recover:
        intel = one_named(jobs, "post-publish-smoke-darwin-x64")
        npm = one_named(intel.get("steps", []),
                        'Run bash scripts/release/smoke/smoke-npm-package.sh "0.8.30"')
        if npm.get("number") != 7:
            raise ValueError("wrong original Intel npm step")
        completed(npm, "success")


def output(*args):
    return subprocess.check_output(args, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version")
    parser.add_argument("run_id")
    parser.add_argument("candidate_sha")
    parser.add_argument("--recover-0830-public-smokes", action="store_true")
    parser.add_argument("--recover-0831-crates-sdk", action="store_true")
    args = parser.parse_args()
    # Validate untrusted dispatch values before using them in refs/API paths.
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.version):
        parser.error("invalid stable version")
    if not re.fullmatch(r"[0-9]+", args.run_id) or not re.fullmatch(r"[0-9a-f]{40}", args.candidate_sha):
        parser.error("invalid run/source")
    repository = os.environ["GITHUB_REPOSITORY"]
    subprocess.run(["git", "fetch", "--no-tags", "--depth=1", "origin", f"refs/tags/v{args.version}"], check=True)
    run = json.loads(output("gh", "api", f"repos/{repository}/actions/runs/{args.run_id}"))
    attempt = run["run_attempt"]
    jobs_json = output("gh", "api", "--paginate", "--jq", ".jobs[] | @json",
                       f"repos/{repository}/actions/runs/{args.run_id}/attempts/{attempt}/jobs?per_page=100")
    validate(
        version=args.version, run_id=args.run_id, candidate_sha=args.candidate_sha,
        recover=args.recover_0830_public_smokes,
        recover_0831_crates_sdk=args.recover_0831_crates_sdk,
        tag_object=output("git", "rev-parse", "FETCH_HEAD"),
        tag_commit=output("git", "rev-parse", "FETCH_HEAD^{commit}"), run=run,
        jobs=[json.loads(line) for line in jobs_json.splitlines()], repository=repository,
    )
    print(f"verified production v{args.version} run={args.run_id} attempt={attempt}; "
          f"affected_smokes_recovery={args.recover_0830_public_smokes}; "
          f"crates_sdk_recovery={args.recover_0831_crates_sdk}")


if __name__ == "__main__":
    main()
