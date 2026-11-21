#!/usr/bin/env python3
"""Reject substituted or incomplete original public-smoke evidence."""

import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
VERSION = "0.8.30"
RUN = "44929252460"
SHA = "e3d31f5cdc0df3e32dab9bad151c5ab5c321222c"
TAG = "fbae0d3319fb31bb62360977fded676f2dea8557"
NPM_STEP = 'Run bash scripts/release/smoke/smoke-npm-package.sh "0.8.30"'
GOOD = [
    "publish-rust-t7-cli", "publish-pypi", "publish-npm",
    "post-publish-smoke (crates-cli)", "post-publish-smoke (crates-sdk)",
    "post-publish-smoke (npm-package)", "post-publish-smoke-aarch64",
    "post-publish-smoke-darwin-x64", "post-publish-smoke-win32-x64",
    "co-tagging-assert",
]


class PublicSmokeRecoveryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location(
            "public_smoke_recovery", ROOT / "scripts/release/verify-public-smoke-recovery.py",
        )
        cls.helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.helper)

    def fixture(self):
        jobs = [{"name": name, "status": "completed", "conclusion": "success"} for name in GOOD]
        intel = next(job for job in jobs if job["name"] == "post-publish-smoke-darwin-x64")
        intel["steps"] = [{"number": 7, "name": NPM_STEP, "status": "completed", "conclusion": "success"}]
        jobs.extend({"name": name, "status": "completed", "conclusion": "failure"} for name in (
            "post-publish-smoke (pypi-wheel)", "post-publish-smoke-darwin-arm64",
        ))
        return dict(
            version=VERSION, run_id=RUN, candidate_sha=SHA, recover=True,
            tag_object=TAG, tag_commit=SHA,
            run={"id": int(RUN), "run_attempt": 1, "event": "push", "head_branch": "v0.8.30",
                 "head_sha": SHA, "path": ".github/workflows/release.yml", "status": "completed",
                 "conclusion": "failure", "repository": {"full_name": "fathomadb/fathomdb"}},
            jobs=jobs, repository="fathomadb/fathomdb",
        )

    def test_exact_original_attempt_accepts_good_proofs_but_not_python_false_green(self):
        self.helper.validate(**self.fixture())

    def test_wrong_identity_and_attempt_refused(self):
        base = self.fixture()
        mutations = [
            {"version": "0.8.31"}, {"run_id": "44929252461"}, {"candidate_sha": "a" * 40},
            {"tag_object": "b" * 40}, {"tag_commit": "b" * 40},
        ]
        for field, value in (
            ("id", 1), ("run_attempt", 2), ("event", "workflow_dispatch"),
            ("head_branch", "main"), ("head_sha", "c" * 40), ("path", "other.yml"),
            ("status", "in_progress"), ("conclusion", "success"),
            ("repository", {"full_name": "other/repo"}),
        ):
            mutations.append({"run": base["run"] | {field: value}})
        for mutation in mutations:
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.helper.validate(**(base | mutation))

    def test_missing_duplicate_or_non_success_good_proof_refused(self):
        for name in GOOD:
            for defect in ("missing", "duplicate", "failure", "skipped", "cancelled", "in_progress"):
                data = self.fixture()
                job = next(job for job in data["jobs"] if job["name"] == name)
                if defect == "missing":
                    data["jobs"].remove(job)
                elif defect == "duplicate":
                    data["jobs"].append(copy.deepcopy(job))
                elif defect == "in_progress":
                    job["status"] = defect
                else:
                    job["conclusion"] = defect
                with self.subTest(name=name, defect=defect), self.assertRaises(ValueError):
                    self.helper.validate(**data)

    def test_green_intel_job_does_not_substitute_for_npm_step(self):
        for defect in ("missing", "duplicate", "skipped", "failure", "in_progress", "wrong_name", "wrong_number"):
            data = self.fixture()
            intel = next(job for job in data["jobs"] if job["name"] == "post-publish-smoke-darwin-x64")
            step = intel["steps"][0]
            if defect == "missing":
                intel["steps"] = []
            elif defect == "duplicate":
                intel["steps"].append(copy.deepcopy(step))
            elif defect == "wrong_name":
                step["name"] = "Python false green"
            elif defect == "wrong_number":
                step["number"] = 6
            elif defect == "in_progress":
                step["status"] = defect
            else:
                step["conclusion"] = defect
            with self.subTest(defect=defect), self.assertRaises(ValueError):
                self.helper.validate(**data)

    def test_default_route_still_requires_all_original_smokes(self):
        data = self.fixture() | {"recover": False}
        with self.assertRaises(ValueError):
            self.helper.validate(**data)
        for job in data["jobs"]:
            job["conclusion"] = "success"
        # The same tag tree always carries the broken macOS Python script.
        with self.assertRaises(ValueError):
            self.helper.validate(**data)
        data["version"] = "0.8.31"
        data["run"]["head_branch"] = "v0.8.31"
        self.helper.validate(**data)


class RecoveryWorkflowTests(unittest.TestCase):
    def test_workflow_requires_recovery_success_before_oidc(self):
        source = (ROOT / ".github/workflows/promote-npm-release.yml").read_text()
        for required in (
            "recover_0830_public_smokes:", "verify-production:", "recover-public-smokes:",
            "needs: [verify-production, recover-public-smokes]",
            "needs.verify-production.result == 'success'",
            "needs.recover-public-smokes.result == 'success'",
            "needs.recover-public-smokes.result == 'skipped'",
            "macos-15-intel", "macos-15", "ubuntu-latest",
            'run: /bin/bash scripts/release/smoke/smoke-pypi-wheel.sh "$VERSION"',
            'run: /bin/bash scripts/release/smoke/smoke-npm-package.sh "$VERSION"',
            "ref: ${{ github.workflow_sha }}",
        ):
            self.assertIn(required, source)


if __name__ == "__main__":
    unittest.main()
