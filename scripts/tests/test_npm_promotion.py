#!/usr/bin/env python3
"""Exercise the exact workflow promotion commands with a controlled npm client."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import yaml


ROOT = Path(__file__).resolve().parents[2]
VERSION = "0.8.30"


class NpmPromotionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.work = Path(self.temporary.name)
        self.calls = self.work / "npm-calls.jsonl"
        npm = self.work / "npm"
        npm.write_text(
            f"#!{sys.executable}\n"
            """import json, os, pathlib, sys, time
args = sys.argv[1:]
path = pathlib.Path(os.environ["TEST_NPM_CALLS"])
old = [json.loads(row) for row in path.read_text().splitlines()] if path.exists() else []
with path.open("a") as out:
    print(json.dumps({
        "argv": args,
        "legacy": "NODE_AUTH_TOKEN" in os.environ or "NPM_TOKEN" in os.environ,
        "oidc": bool(os.environ.get("ACTIONS_ID_TOKEN_REQUEST_TOKEN")),
    }), file=out)
if "--version" in args:
    print(os.environ.get("TEST_NPM_VERSION", "11.21.0"))
elif "dist-tag" in args:
    print("+latest: fathomdb@0.8.30")
elif "view" in args:
    previous = sum("view" in row["argv"] for row in old)
    scenario = os.environ.get("TEST_NPM_READ", "eventual")
    if scenario == "slow":
        time.sleep(5)
    if scenario == "transport":
        print("npm error code ECONNRESET; secret-error-payload", file=sys.stderr)
        sys.exit(42)
    if scenario == "missing":
        sys.exit(0)
    if scenario == "invalid":
        print(json.dumps("::error::secret-error-payload"))
    elif scenario == "stale" or previous < 2:
        print(json.dumps("0.8.29") if "--json" in args else "0.8.29")
    else:
        print(json.dumps("0.8.30") if "--json" in args else "0.8.30")
else:
    raise AssertionError(args)
""",
            encoding="utf-8",
        )
        npm.chmod(0o755)
        self.env = os.environ | {
            "PATH": f"{self.work}{os.pathsep}{os.environ['PATH']}",
            "TEST_NPM_CALLS": str(self.calls),
            "NODE_AUTH_TOKEN": "synthetic-legacy-fixture",
            "NPM_TOKEN": "synthetic-legacy-fixture",
            "ACTIONS_ID_TOKEN_REQUEST_URL": "https://oidc.fixture.invalid",
            "ACTIONS_ID_TOKEN_REQUEST_TOKEN": "synthetic-oidc-fixture",
            "RELEASE_TAG": "v0.8.30",
            "VERSION": VERSION,
        }

    def records(self):
        if not self.calls.exists():
            return []
        return [json.loads(row) for row in self.calls.read_text().splitlines()]

    def test_shared_job_routes_observe_latest_before_success(self):
        routes = []
        for workflow in ("release.yml", "promote-npm-release.yml"):
            with self.subTest(workflow=workflow):
                self.calls.unlink(missing_ok=True)
                source = yaml.safe_load((ROOT / ".github/workflows" / workflow).read_text())
                job = source["jobs"]["promote-npm-latest"]
                step, = [
                    step["run"] for step in job["steps"]
                    if "dist-tag add" in step.get("run", "")
                    or "promote-npm-latest.py" in step.get("run", "")
                ]
                routes.append(step.split()[1] if step.startswith("python3 ") else step)
                result = subprocess.run(
                    ["bash", "-c", step], cwd=ROOT, env=self.env,
                    capture_output=True, text=True, timeout=30, check=False,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                calls = self.records()
                writes = [row for row in calls if "dist-tag" in row["argv"]]
                reads = [row for row in calls if "view" in row["argv"]]
                self.assertEqual(len(writes), 1)
                argv = writes[0]["argv"]
                index = argv.index("dist-tag")
                self.assertEqual(argv[index:index + 4], ["dist-tag", "add", f"fathomdb@{VERSION}", "latest"])
                self.assertFalse(writes[0]["legacy"])
                self.assertTrue(writes[0]["oidc"])
                self.assertEqual(len(reads), 3)
                self.assertTrue(all("--prefer-online" in row["argv"] for row in reads))
                self.assertIn("expected=0.8.30 observed=0.8.29", result.stdout)
                self.assertIn("expected=0.8.30 observed=0.8.30", result.stdout)
                self.assertEqual(job["permissions"]["id-token"], "write")
                self.assertNotIn("id-token", source.get("permissions", {}))
                self.assertTrue(any(
                    step.get("with", {}).get("ref") == "${{ github.workflow_sha }}"
                    and step.get("with", {}).get("persist-credentials") is False
                    for step in job["steps"] if step.get("uses", "").startswith("actions/checkout@")
                ))
        self.assertEqual(len(set(routes)), 1, routes)

    def load_helper(self):
        spec = importlib.util.spec_from_file_location(
            "npm_promotion", ROOT / "scripts/release/promote-npm-latest.py",
        )
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        return helper

    def test_readback_can_fail_without_a_second_mutation(self):
        helper = self.load_helper()
        for scenario, diagnostic in (
            ("stale", "observed=0.8.29"),
            ("missing", "observed=missing"),
            ("invalid", "observed=invalid-metadata"),
            ("transport", "observed=lookup-error(exit=42)"),
        ):
            with self.subTest(scenario=scenario):
                self.calls.unlink(missing_ok=True)
                with (
                    patch.dict(os.environ, self.env | {"TEST_NPM_READ": scenario}, clear=True),
                    patch.object(helper.time, "sleep"),
                    patch("sys.argv", ["promote-npm-latest.py", VERSION]),
                    patch("sys.stdout") as stdout,
                ):
                    self.assertEqual(helper.main(), 1)
                output = "".join(str(call.args[0]) for call in stdout.write.call_args_list)
                self.assertIn(diagnostic, output)
                self.assertNotIn("secret-error-payload", output)
                self.assertEqual(sum("dist-tag" in row["argv"] for row in self.records()), 1)
                self.assertEqual(sum("view" in row["argv"] for row in self.records()), 6)

    def test_elapsed_budget_includes_the_registry_lookup(self):
        helper = self.load_helper()
        with (
            patch.dict(os.environ, self.env | {"TEST_NPM_READ": "slow"}, clear=True),
            patch.object(helper, "READBACK_SECONDS", 0.25),
            patch("sys.argv", ["promote-npm-latest.py", VERSION]),
        ):
            started = helper.time.monotonic()
            self.assertEqual(helper.main(), 1)
            self.assertLess(helper.time.monotonic() - started, 2)
        self.assertEqual(sum("dist-tag" in row["argv"] for row in self.records()), 1)
        self.assertEqual(sum("view" in row["argv"] for row in self.records()), 1)

    def test_invalid_pin_scope_and_missing_oidc_never_write(self):
        helper = self.load_helper()
        for changes, version in (
            ({"TEST_NPM_VERSION": "11.20.0"}, VERSION),
            ({"ACTIONS_ID_TOKEN_REQUEST_TOKEN": ""}, VERSION),
            ({}, "next"),
        ):
            with self.subTest(changes=changes, version=version):
                self.calls.unlink(missing_ok=True)
                with (
                    patch.dict(os.environ, self.env | changes, clear=True),
                    patch("sys.argv", ["promote-npm-latest.py", version]),
                ):
                    self.assertNotEqual(helper.main(), 0)
                self.assertFalse(any("dist-tag" in row["argv"] for row in self.records()))


if __name__ == "__main__":
    unittest.main()
