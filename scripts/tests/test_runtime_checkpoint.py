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
EVIDENCE_DIR = Path("dev/plans/0.8.27/features/slice-90")
MATRIX_CELLS = ("2/1", "1/1", "2/2", "4/4", "64/64", "2/no-provider")
PROTOCOL = json.loads((REPO_ROOT / EVIDENCE_DIR / "d27-runtime-qualification-protocol.json").read_text())
RELEASE_SELECTORS = ("AC-011a", "AC-011b", "AC-017", "AC-018", "AC-029", "AC-072", "AC-073", "AC-076", "AC-081a", "AC-081b", "AC-081c")


class RuntimeCheckpointGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        (self.root / EVIDENCE_DIR).mkdir(parents=True)

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
            "release": "0.8.27",
            "ladder": [
                {
                    "slice": 90,
                    "status": "PLANNED",
                    "runtime_checkpoint": checkpoint,
                }
            ],
        }
        (self.root / "dev" / "plans" / "release-state-0.8.27.json").write_text(
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
                "performance": self.pending_receipt(str(EVIDENCE_DIR / "runtime-performance-qualification.md")),
                "code_review": self.pending_receipt(str(EVIDENCE_DIR / "code-review.md")),
                "verification": self.pending_receipt(str(EVIDENCE_DIR / "review-verification.md")),
            },
        }

    def test_pending_checkpoint_with_predeclared_receipts_passes(self) -> None:
        self.write_state(self.pending_checkpoint())
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

    def write_performance(self, candidate: str, cells: tuple[str, ...] = MATRIX_CELLS, d27: bool = True) -> str:
        rows = "\n".join(f"| {cell} | PASS | {candidate} | cargo test -p fathomdb-engine slice90_matrix_{cell.replace('/', '_').replace('-', '_')} | 3 passed; exact resource inventory and cleanup |" for cell in cells)
        qualification = ""
        if d27:
            d27_path = EVIDENCE_DIR / "d27-candidate-receipt.json"
            per_repetition = {}
            aggregates = {}
            for direction in ("projection_heavy", "foreground_heavy"):
                run = {
                    "environment_valid": True, "starvation_pass": True,
                    "swap_pages_in_delta": 0, "swap_pages_out_delta": 0,
                    "counts": {name: 1 for name in PROTOCOL["metrics"]["counts"]},
                    "high_water": {name: 1 for name in PROTOCOL["metrics"]["high_water"]},
                    "inventory": {name: 1 for name in PROTOCOL["metrics"]["inventory"]},
                    "throughput": {name: 10.0 for name in PROTOCOL["metrics"]["throughput_rates_per_second"]},
                    "latency_ms": {name: {str(p): 1.0 for p in PROTOCOL["metrics"]["latency_percentiles"]}
                                   for name in PROTOCOL["metrics"]["latency_classes"] + PROTOCOL["metrics"]["candidate_only_latency_classes"]},
                }
                per_repetition[direction] = [run.copy() for _ in range(3)]
                aggregates[direction] = {
                    "throughput": {name: {"median": 10.0, "mad": 0.0} for name in PROTOCOL["metrics"]["throughput_rates_per_second"]},
                    "latency_ms": {name: {str(p): {"median": 1.0, "mad": 0.0} for p in PROTOCOL["metrics"]["latency_percentiles"]}
                                   for name in PROTOCOL["metrics"]["latency_classes"]},
                }
            d27_receipt = {
                "phase": "candidate",
                "source_sha": candidate,
                "status": "PASS",
                "protocol_sha256": hashlib.sha256((REPO_ROOT / EVIDENCE_DIR / "d27-runtime-qualification-protocol.json").read_bytes()).hexdigest(),
                "binary_sha256": "a" * 64, "runner_sha256": "b" * 64,
                "corpus_sha256": "c" * 64, "raw_output_sha256": "d" * 64,
                "runner_inventory": {"host": "windchill3"},
                "environment_start": {"cpu_governor": "performance"},
                "environment_end": {"cpu_governor": "performance"},
                "per_repetition_metrics": per_repetition,
                "decision_rule_evaluation": {"status": "PASS", "rule": "D27 frozen median/MAD"},
                "aggregate_metrics": aggregates,
            }
            artifact = self.root / d27_path
            artifact.write_text(json.dumps(d27_receipt), encoding="utf-8")
            qualification = (
                "\n## D27 candidate qualification\n\n"
                "| Receipt path | SHA-256 | Status |\n| --- | --- | --- |\n"
                f"| {d27_path} | {hashlib.sha256(artifact.read_bytes()).hexdigest()} | PASS |\n"
            )
        return (
            "# Runtime performance qualification\n\n"
            "## Configuration matrix\n\n"
            "| Cell | Status | Candidate SHA | Command | Evidence |\n| --- | --- | --- | --- | --- |\n"
            f"{rows}\n\n"
            "## Named release selectors\n\n"
            "| Selector | Status | Candidate SHA | Command | Evidence |\n| --- | --- | --- | --- | --- |\n"
            + "\n".join(f"| {selector} | PASS | {candidate} | cargo test -p fathomdb-engine {selector} | 1 passed; unchanged selector |" for selector in RELEASE_SELECTORS)
            + "\n\n## Installed bindings\n\n"
            "| Language | Status | Candidate SHA | Command | Evidence |\n| --- | --- | --- | --- | --- |\n"
            f"| Python | PASS | {candidate} | python3 -m pytest installed_binding | 36 passed; clean consumer install |\n"
            f"| Node | PASS | {candidate} | npm test -- installed_binding | 18 passed; clean consumer install |\n"
            + qualification
        )

    def pass_checkpoint(self, *, cells: tuple[str, ...] = MATRIX_CELLS, d27: bool = True) -> dict[str, object]:
        def git(*args: str) -> str:
            return subprocess.run(["git", "-C", str(self.root), *args], check=True, text=True, capture_output=True).stdout.strip()

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
            if name == "performance":
                body = self.write_performance(candidate, cells, d27)
            else:
                reviewer = "gpt-6-sol high" if name == "code_review" else "Terra"
                body = (f"# {name}\n\nCandidate SHA: {candidate}\nVerdict: PASS\nReviewer: {reviewer}\n"
                        "Evidence: Independent review of the exact candidate found no open findings; "
                        "focused runtime and checkpoint tests passed with complete receipts.\n")
            path.write_text(body, encoding="utf-8")
            receipt.update(status="PASS", candidate_sha=candidate, sha256=hashlib.sha256(path.read_bytes()).hexdigest())
        git("add", "dev/plans/0.8.27/features/slice-90")
        git("commit", "-qm", "bind checkpoint receipts")
        checkpoint.update(status="PASS", candidate_sha=candidate, binding_sha=git("rev-parse", "HEAD"))
        self.write_state(checkpoint)
        return checkpoint

    def test_complete_pass_checkpoint_accepts_required_matrix_and_d27(self) -> None:
        self.pass_checkpoint()
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_pass_performance_requires_every_matrix_cell(self) -> None:
        self.pass_checkpoint(cells=MATRIX_CELLS[:-1])
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("missing matrix cell: 2/no-provider", result.stdout)

    def test_pass_performance_requires_candidate_d27_qualification(self) -> None:
        self.pass_checkpoint(d27=False)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27 candidate qualification", result.stdout)

    def test_receipt_path_outside_declared_slice90_evidence_set_is_rejected(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt = checkpoint["receipts"]["performance"]  # type: ignore[index]
        outside = self.root / "dev/plans/other-performance.md"
        outside.write_bytes((self.root / str(receipt["path"])).read_bytes())
        receipt["path"] = "dev/plans/other-performance.md"
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("outside declared Slice 90 evidence set", result.stdout)

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

        checkpoint = self.pass_checkpoint()
        candidate = checkpoint["candidate_sha"]

        (self.root / "stage3.txt").write_text("stage3\n", encoding="utf-8")
        git("add", "stage3.txt")
        git("commit", "-qm", "stage3")
        stage3 = git("rev-parse", "HEAD")
        checkpoint["stage3_start_sha"] = stage3
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

        checkpoint["stage3_start_sha"] = candidate
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("binding commit must be an ancestor", result.stdout)

    def test_unrelated_checkpoint_cannot_mask_missing_slice90_checkpoint(self) -> None:
        state = {"release": "0.8.27", "ladder": [{"slice": 90, "status": "PLANNED"},
                  {"slice": 80, "runtime_checkpoint": self.pending_checkpoint()}]}
        (self.root / "dev/plans/release-state-0.8.27.json").write_text(json.dumps(state))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("slice 90 runtime_checkpoint missing", result.stdout)

    def test_minimal_d27_pass_payload_is_rejected(self) -> None:
        checkpoint = self.pass_checkpoint()
        path = self.root / EVIDENCE_DIR / "d27-candidate-receipt.json"
        receipt = json.loads(path.read_text())
        receipt["aggregate_metrics"] = {"projection_heavy": {}, "foreground_heavy": {}}
        path.write_text(json.dumps(receipt))
        self._rebind_performance(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27 candidate", result.stdout)

    def test_one_word_review_and_verification_are_rejected(self) -> None:
        checkpoint = self.pass_checkpoint()
        for name in ("code_review", "verification"):
            path = self.root / checkpoint["receipts"][name]["path"]
            path.write_text("PASS\n")
            checkpoint["receipts"][name]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        self._commit_rebound(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("code_review receipt lacks candidate-bound review evidence", result.stdout)
        self.assertIn("verification receipt lacks candidate-bound review evidence", result.stdout)

    def test_duplicate_or_placeholder_matrix_rows_are_rejected(self) -> None:
        checkpoint = self.pass_checkpoint(cells=MATRIX_CELLS + ("2/1",))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate matrix cell: 2/1", result.stdout)

        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        content = path.read_text().replace("| PASS | " + checkpoint["candidate_sha"], "| PASS | " + "f" * 40, 1)
        path.write_text(content)
        self._rebind_performance(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("matrix cell candidate mismatch", result.stdout)

    def test_structural_source_change_requires_stage3_start_sha(self) -> None:
        checkpoint = self.pass_checkpoint()
        source = self.root / "src/rust/crates/fathomdb-engine/src"
        source.mkdir(parents=True)
        (source / "open.rs").write_text("// structural move\n")
        subprocess.run(["git", "-C", str(self.root), "add", "src"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "first structural move"], check=True)
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stage3_start_sha missing after engine source change", result.stdout)

    def test_performance_requires_named_selectors_and_installed_bindings(self) -> None:
        checkpoint = self.pass_checkpoint()
        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        content = path.read_text()
        content = content.replace("| AC-072 | PASS", "| AC-072 | PENDING")
        content = content.replace("| Node | PASS", "| Node | PENDING")
        path.write_text(content)
        checkpoint["receipts"]["performance"]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        self._commit_rebound(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("missing named selector: AC-072", result.stdout)
        self.assertIn("missing installed binding: Node", result.stdout)

    def test_stage3_marker_must_name_first_engine_source_commit(self) -> None:
        checkpoint = self.pass_checkpoint()
        source = self.root / "src/rust/crates/fathomdb-engine/src"
        source.mkdir(parents=True)
        for name in ("open.rs", "connection_runtime.rs"):
            (source / name).write_text("// structural move\n")
            subprocess.run(["git", "-C", str(self.root), "add", "src"], check=True)
            subprocess.run(["git", "-C", str(self.root), "commit", "-qm", f"move {name}"], check=True)
        checkpoint["stage3_start_sha"] = subprocess.run(
            ["git", "-C", str(self.root), "rev-parse", "HEAD"], check=True, text=True, capture_output=True
        ).stdout.strip()
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stage3_start_sha must identify first engine source change", result.stdout)

    def _commit_rebound(self, checkpoint: dict[str, object]) -> None:
        subprocess.run(["git", "-C", str(self.root), "add", "dev/plans/0.8.27/features/slice-90"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "rebind receipts"], check=True)
        checkpoint["binding_sha"] = subprocess.run(["git", "-C", str(self.root), "rev-parse", "HEAD"], check=True, capture_output=True, text=True).stdout.strip()
        self.write_state(checkpoint)

    def _rebind_performance(self, checkpoint: dict[str, object]) -> None:
        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        d27 = self.root / EVIDENCE_DIR / "d27-candidate-receipt.json"
        lines = path.read_text().splitlines()
        for index, line in enumerate(lines):
            if line.startswith(f"| {EVIDENCE_DIR / 'd27-candidate-receipt.json'} |"):
                lines[index] = f"| {EVIDENCE_DIR / 'd27-candidate-receipt.json'} | {hashlib.sha256(d27.read_bytes()).hexdigest()} | PASS |"
        path.write_text("\n".join(lines) + "\n")
        checkpoint["receipts"]["performance"]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        self._commit_rebound(checkpoint)


if __name__ == "__main__":
    unittest.main()
