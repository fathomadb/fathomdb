#!/usr/bin/env python3
"""Behavioral tests for the runtime-checkpoint release-state gate."""

from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
GATE = REPO_ROOT / "scripts" / "check-runtime-checkpoints.py"


class RuntimeCheckpointGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        (self.root / "dev" / "plans" / "receipts").mkdir(parents=True)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def run_gate(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["python3", str(GATE), "--root", str(self.root)],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def write_state(self, checkpoint: dict[str, object]) -> None:
        state = {
            "release": "9.9.9",
            "ladder": [
                {
                    "slice": 90,
                    "status": "PLANNED",
                    "runtime_checkpoint": checkpoint,
                }
            ],
        }
        (self.root / "dev" / "plans" / "release-state-9.9.9.json").write_text(
            json.dumps(state), encoding="utf-8"
        )

    @staticmethod
    def pending_receipt(path: str) -> dict[str, object]:
        return {"path": path, "status": "PENDING", "candidate_sha": None, "sha256": None}

    def pending_checkpoint(self) -> dict[str, object]:
        return {
            "status": "PENDING",
            "candidate_sha": None,
            "binding_sha": None,
            "stage3_start_sha": None,
            "receipts": {
                "performance": self.pending_receipt("dev/plans/receipts/performance.md"),
                "code_review": self.pending_receipt("dev/plans/receipts/code-review.md"),
                "verification": self.pending_receipt("dev/plans/receipts/verification.md"),
            },
        }

    def test_pending_checkpoint_with_predeclared_receipts_passes(self) -> None:
        self.write_state(self.pending_checkpoint())
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_zero_structured_checkpoints_fails_closed(self) -> None:
        state = {"release": "9.9.9", "ladder": [{"slice": 90, "status": "PLANNED"}]}
        (self.root / "dev" / "plans" / "release-state-9.9.9.json").write_text(
            json.dumps(state), encoding="utf-8"
        )
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("zero structured checkpoints", result.stdout)

    def test_stage3_cannot_start_while_checkpoint_is_pending(self) -> None:
        checkpoint = self.pending_checkpoint()
        checkpoint["stage3_start_sha"] = "a" * 40
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stage3_start_sha requires a PASS checkpoint", result.stdout)

    def test_pass_checkpoint_rejects_missing_receipt(self) -> None:
        checkpoint = self.pending_checkpoint()
        checkpoint.update(
            status="PASS",
            candidate_sha="a" * 40,
            binding_sha="b" * 40,
        )
        for receipt in checkpoint["receipts"].values():  # type: ignore[union-attr]
            receipt.update(status="PASS", candidate_sha="a" * 40, sha256="0" * 64)
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("receipt file does not exist", result.stdout)

    def test_pass_checkpoint_rejects_receipt_hash_or_candidate_drift(self) -> None:
        checkpoint = self.pending_checkpoint()
        checkpoint.update(
            status="PASS",
            candidate_sha="a" * 40,
            binding_sha="b" * 40,
        )
        for name, receipt in checkpoint["receipts"].items():  # type: ignore[union-attr]
            path = self.root / str(receipt["path"])
            path.write_text(f"{name}\n", encoding="utf-8")
            receipt.update(
                status="PASS",
                candidate_sha="a" * 40,
                sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            )
        checkpoint["receipts"]["verification"]["candidate_sha"] = "c" * 40  # type: ignore[index]
        checkpoint["receipts"]["code_review"]["sha256"] = "d" * 64  # type: ignore[index]
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("code_review receipt sha256 mismatch", result.stdout)
        self.assertIn("verification receipt candidate_sha", result.stdout)

    def test_pass_checkpoint_requires_resolving_candidate_and_binding_commits(self) -> None:
        checkpoint = self.pending_checkpoint()
        checkpoint.update(
            status="PASS",
            candidate_sha="a" * 40,
            binding_sha="b" * 40,
        )
        for name, receipt in checkpoint["receipts"].items():  # type: ignore[union-attr]
            path = self.root / str(receipt["path"])
            path.write_text(f"{name}\n", encoding="utf-8")
            receipt.update(
                status="PASS",
                candidate_sha="a" * 40,
                sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            )
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("candidate_sha does not resolve to a commit", result.stdout)
        self.assertIn("binding_sha does not resolve to a commit", result.stdout)

    def test_stage3_commit_must_descend_from_checkpoint_binding(self) -> None:
        def git(*args: str) -> str:
            result = subprocess.run(
                ["git", "-C", str(self.root), *args],
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            )
            return result.stdout.strip()

        git("init", "-q")
        git("config", "user.email", "runtime-checkpoint@example.invalid")
        git("config", "user.name", "Runtime Checkpoint Test")
        (self.root / "candidate.txt").write_text("candidate\n", encoding="utf-8")
        git("add", "candidate.txt")
        git("commit", "-qm", "candidate")
        candidate = git("rev-parse", "HEAD")

        checkpoint = self.pending_checkpoint()
        for name, receipt in checkpoint["receipts"].items():  # type: ignore[union-attr]
            path = self.root / str(receipt["path"])
            path.write_text(f"{name}\n", encoding="utf-8")
            receipt.update(
                status="PASS",
                candidate_sha=candidate,
                sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            )
        git("add", "dev/plans/receipts")
        git("commit", "-qm", "bind checkpoint receipts")
        binding = git("rev-parse", "HEAD")

        (self.root / "stage3.txt").write_text("stage3\n", encoding="utf-8")
        git("add", "stage3.txt")
        git("commit", "-qm", "stage3")
        stage3 = git("rev-parse", "HEAD")
        checkpoint.update(
            status="PASS",
            candidate_sha=candidate,
            binding_sha=binding,
            stage3_start_sha=stage3,
        )
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

        checkpoint["stage3_start_sha"] = candidate
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("binding commit must be an ancestor", result.stdout)


if __name__ == "__main__":
    unittest.main()
