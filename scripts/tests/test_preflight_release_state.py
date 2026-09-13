#!/usr/bin/env python3
"""Focused RED/GREEN tests for Slice 72 release-state preflight facts."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "preflight-release-state.py"
SPEC = importlib.util.spec_from_file_location("preflight_release_state", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def git(root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout.strip()


class Fixture:
    def __init__(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.email", "slice72@example.invalid")
        git(self.root, "config", "user.name", "Slice 72")
        git(self.root, "config", "commit.gpgsign", "false")
        (self.root / "seed").write_text("seed\n", encoding="utf-8")
        git(self.root, "add", "seed")
        git(self.root, "commit", "-q", "-m", "seed")
        self.dependency_sha = git(self.root, "rev-parse", "HEAD")
        git(self.root, "checkout", "-q", "-b", "release/9.1.0")
        self.board = "dev/plans/runs/STATUS-9.1.0.md"
        self.plan = "dev/plans/plan-9.1.0.md"
        self.state = "dev/plans/release-state-9.1.0.json"
        for relative in [self.board, self.plan, self.state]:
            (self.root / relative).parent.mkdir(parents=True, exist_ok=True)
        (self.root / self.board).write_text("# live\n", encoding="utf-8")
        (self.root / self.plan).write_text("# plan\n", encoding="utf-8")
        self.data = {
            "release": "9.1.0",
            "board": self.board,
            "plan": self.plan,
            "active_ref": "refs/heads/release/9.1.0",
            "ladder": [
                {
                    "slice": 71,
                    "status": "COMPLETE_ON_RELEASE_BRANCH",
                    "sha": self.dependency_sha,
                }
            ],
        }
        self.write_state()
        git(self.root, "add", "dev")
        git(self.root, "commit", "-q", "-m", "state")

    def write_state(self) -> None:
        (self.root / self.state).write_text(
            json.dumps(self.data, sort_keys=True) + "\n", encoding="utf-8"
        )

    def validate(self, *, target_head: str | None = None) -> dict:
        return MODULE.validate_state(
            repo_root=self.root,
            release="9.1.0",
            board=self.board,
            state_file=self.state,
            plan=self.plan,
            expect_closed=71,
            target_head=target_head or git(self.root, "rev-parse", "HEAD"),
        )

    def close(self) -> None:
        self.temp.cleanup()


class PreflightReleaseStateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = Fixture()

    def tearDown(self) -> None:
        self.fixture.close()

    def test_active_release_and_dependency_only_head_pass(self) -> None:
        facts = self.fixture.validate()
        self.assertEqual(facts["baseline_ref"], "refs/heads/release/9.1.0")
        self.assertEqual(facts["dependency_sha"], self.fixture.dependency_sha)

    def test_wrong_active_ref_fails(self) -> None:
        self.fixture.data["active_ref"] = "refs/heads/release/9.1.1"
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "active_ref"):
            self.fixture.validate()

    def test_plan_identity_fails_closed(self) -> None:
        self.fixture.data["plan"] = "dev/plans/wrong.md"
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "plan"):
            self.fixture.validate()

    def test_plan_path_spellings_normalize_but_escape_fails(self) -> None:
        for spelling in [
            str(self.fixture.root / self.fixture.plan),
            "dev/plans/../plans/plan-9.1.0.md",
        ]:
            facts = MODULE.validate_state(
                repo_root=self.fixture.root,
                release="9.1.0",
                board=str(self.fixture.root / self.fixture.board),
                state_file=str(self.fixture.root / self.fixture.state),
                plan=spelling,
                expect_closed=71,
                target_head=git(self.fixture.root, "rev-parse", "HEAD"),
            )
            self.assertEqual(facts["state_file"], self.fixture.state)
        with self.assertRaisesRegex(MODULE.StateError, "escapes the repository"):
            MODULE.validate_state(
                repo_root=self.fixture.root,
                release="9.1.0",
                board=self.fixture.board,
                state_file=self.fixture.state,
                plan="../outside.md",
                expect_closed=71,
                target_head=git(self.fixture.root, "rev-parse", "HEAD"),
            )

    def test_dependency_must_be_unique_and_closed(self) -> None:
        self.fixture.data["ladder"] = []
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "exactly one"):
            self.fixture.validate()
        self.fixture.data["ladder"] = [
            {
                "slice": 71,
                "status": "COMPLETE_ON_RELEASE_BRANCH",
                "sha": self.fixture.dependency_sha,
            }
        ]
        self.fixture.data["ladder"].append(dict(self.fixture.data["ladder"][0]))
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "exactly one"):
            self.fixture.validate()

    def test_malformed_state_fails_closed(self) -> None:
        (self.fixture.root / self.fixture.state).write_text("{not json\n", encoding="utf-8")
        with self.assertRaisesRegex(MODULE.StateError, "cannot parse release state"):
            self.fixture.validate()
        self.fixture.data["ladder"] = [self.fixture.data["ladder"][0]]
        self.fixture.data["ladder"][0]["status"] = "NOT_STARTED"
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "not closed"):
            self.fixture.validate()

    def test_dependency_sha_must_resolve_and_be_ancestor(self) -> None:
        self.fixture.data["ladder"][0]["sha"] = "deadbeef"
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "does not resolve"):
            self.fixture.validate()

        self.fixture.data["ladder"][0]["sha"] = self.fixture.dependency_sha
        self.fixture.write_state()
        git(self.fixture.root, "checkout", "-q", "--orphan", "unrelated")
        (self.fixture.root / "other").write_text("other\n", encoding="utf-8")
        git(self.fixture.root, "add", "other")
        git(self.fixture.root, "commit", "-q", "-m", "unrelated")
        target = git(self.fixture.root, "rev-parse", "HEAD")
        with self.assertRaisesRegex(MODULE.StateError, "target HEAD"):
            self.fixture.validate(target_head=target)

    def test_dependency_ancestry_is_checked_independently(self) -> None:
        git(self.fixture.root, "checkout", "-q", "--orphan", "dependency-side")
        (self.fixture.root / "side").write_text("side\n", encoding="utf-8")
        git(self.fixture.root, "add", "side")
        git(self.fixture.root, "commit", "-q", "-m", "side")
        side_sha = git(self.fixture.root, "rev-parse", "HEAD")
        self.fixture.data["ladder"][0]["sha"] = side_sha
        self.fixture.write_state()
        target = git(self.fixture.root, "rev-parse", "release/9.1.0")
        with self.assertRaisesRegex(MODULE.StateError, "dependency Slice 71 SHA"):
            self.fixture.validate(target_head=target)

    def test_pending_and_complete_completion_lifecycles(self) -> None:
        release_head = git(self.fixture.root, "rev-parse", "release/9.1.0")
        git(
            self.fixture.root,
            "update-ref",
            "refs/remotes/origin/release/9.1.0",
            release_head,
        )
        self.fixture.data["completion"] = {
            "ref": "origin/release/9.1.0",
            "main_integration": "PENDING",
        }
        self.fixture.write_state()
        self.assertEqual(
            self.fixture.validate()["baseline_ref"], "origin/release/9.1.0"
        )

        git(self.fixture.root, "update-ref", "refs/remotes/origin/main", release_head)
        self.fixture.data["completion"]["main_integration"] = "COMPLETE"
        self.fixture.write_state()
        self.assertEqual(self.fixture.validate()["baseline_ref"], "origin/main")

    def test_completion_lifecycle_fails_closed(self) -> None:
        self.fixture.data["completion"] = {
            "ref": "origin/release/9.1.0",
            "main_integration": "PENDING",
        }
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "does not resolve"):
            self.fixture.validate()

        self.fixture.data["completion"] = {
            "ref": "origin/release/WRONG",
            "main_integration": "PENDING",
        }
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "completion.ref"):
            self.fixture.validate()

        release_head = git(self.fixture.root, "rev-parse", "release/9.1.0")
        git(self.fixture.root, "update-ref", "refs/remotes/origin/release/9.1.0", release_head)
        git(self.fixture.root, "checkout", "-q", "--orphan", "main-side")
        (self.fixture.root / "main-side").write_text("main side\n", encoding="utf-8")
        git(self.fixture.root, "add", "main-side")
        git(self.fixture.root, "commit", "-q", "-m", "main side")
        git(self.fixture.root, "update-ref", "refs/remotes/origin/main", "HEAD")
        self.fixture.data["completion"] = {
            "ref": "origin/release/9.1.0",
            "main_integration": "COMPLETE",
        }
        self.fixture.write_state()
        with self.assertRaisesRegex(MODULE.StateError, "not reachable"):
            self.fixture.validate(target_head=release_head)


class RealPreflightRegressionTest(unittest.TestCase):
    def test_current_release_worktree_and_slice8_are_accepted(self) -> None:
        result = subprocess.run(
            [
                "bash",
                str(ROOT / "scripts" / "preflight.sh"),
                "--worktree",
                str(ROOT),
                "--expect-closed",
                "8",
                "--plan",
                "dev/plans/plan-0.8.26.md",
                "--min-disk-gb",
                "1",
            ],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn('"release":"0.8.26"', result.stdout)


if __name__ == "__main__":
    unittest.main()
