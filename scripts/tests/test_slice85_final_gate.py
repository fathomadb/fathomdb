#!/usr/bin/env python3
"""Mutation tests for the Slice 85 evidence gate and installed-runtime route."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "scripts/release/verify-slice85-manifest.py"
LEGACY = ROOT / "dev/plans/0.8.25/features/slice-75/slice75-closure-manifest.json"
CE_BASE = ROOT / "dev/plans/0.8.25/features/slice-72/ce-profile-manifest.json"
SMOKE = ROOT / "scripts/release/smoke/smoke-local-native-artifacts.sh"
HASH = "2" * 64


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Slice85FinalGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="slice85-gate-test-")
        self.repo = Path(self.temporary.name)
        legacy_path = self.repo / LEGACY.relative_to(ROOT)
        legacy_path.parent.mkdir(parents=True)
        legacy_path.write_bytes(LEGACY.read_bytes())
        ce_base_path = self.repo / CE_BASE.relative_to(ROOT)
        ce_base_path.parent.mkdir(parents=True)
        ce_base_path.write_bytes(CE_BASE.read_bytes())
        overlay = json.loads(CE_BASE.read_text(encoding="utf-8"))
        overlay_path = (
            self.repo / "dev/plans/runs/0.8.25-slice-85/slice72-ce-manifest.json"
        )
        overlay_path.parent.mkdir(parents=True)
        (self.repo / "source-input.txt").write_text(
            "candidate input\n", encoding="utf-8"
        )
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        subprocess.run(
            ["git", "-C", str(self.repo), "config", "user.name", "Slice 85 Test"],
            check=True,
        )
        subprocess.run(
            [
                "git",
                "-C",
                str(self.repo),
                "config",
                "user.email",
                "slice85@example.invalid",
            ],
            check=True,
        )
        subprocess.run(["git", "-C", str(self.repo), "add", "."], check=True)
        subprocess.run(
            ["git", "-C", str(self.repo), "commit", "-qm", "fixture"], check=True
        )
        self.candidate_sha = subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "HEAD"], text=True
        ).strip()
        self.candidate_tree = subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "HEAD^{tree}"], text=True
        ).strip()
        overlay["candidate_sha"] = self.candidate_sha
        overlay_path.write_text(json.dumps(overlay), encoding="utf-8")
        self.input_hash = digest(self.repo / "source-input.txt")
        self.artifacts = {}
        for name in ("python-wheel", "napi-linux-x64-gnu", "cli-linux-x64-gnu"):
            path = self.repo / "artifacts" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(name, encoding="utf-8")
            self.artifacts[name] = {
                "path": str(path.relative_to(self.repo)),
                "sha256": digest(path),
            }

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def obligation(
        self,
        obligation_id: str,
        origin: str = "legacy",
        commands: list[str] | None = None,
    ) -> dict:
        return {
            "id": obligation_id,
            "origin": origin,
            "disposition": "run",
            "candidate_sha": self.candidate_sha,
            "input_paths": ["source-input.txt"],
            "current_input_sha256": self.input_hash,
            "accepted_input_sha256": None,
            "commands": commands if commands is not None else ["true"],
            "retained_receipt": None,
            "artifact_sha256": None,
            "evidence": [],
            "verdict": "pending",
        }

    def manifest(self) -> dict:
        legacy = json.loads(LEGACY.read_text(encoding="utf-8"))
        obligations = [
            self.obligation(cell["id"], commands=cell["commands"])
            for cell in legacy["cells"]
        ]
        obligations.extend(
            [
                self.obligation(
                    "runtime-configuration",
                    "additional",
                    ["runtime source", "runtime installed"],
                ),
                self.obligation(
                    "protected-writes",
                    "additional",
                    [f"protected write {index}" for index in range(7)],
                ),
                self.obligation(
                    "slice72-ce",
                    "additional",
                    [
                        "slice72 artifact",
                        "slice72 cpu",
                        "slice72 cuda",
                        "slice72 verify",
                    ],
                ),
            ]
        )
        ac034c = self.obligation("ac034c", "authorized-exception")
        ac034c.update(
            disposition="unavailable",
            commands=[],
            evidence=[],
            verdict="unavailable",
        )
        obligations.append(ac034c)
        return {
            "schema_version": "fathomdb.slice85-final-manifest/v1",
            "release": "0.8.25",
            "branch": "release/0.8.25",
            "candidate": {
                "sha": self.candidate_sha,
                "tree": self.candidate_tree,
                "artifacts": self.artifacts,
            },
            "legacy_manifest": {
                "path": str(LEGACY.relative_to(ROOT)),
                "sha256": digest(LEGACY),
            },
            "ce_overlay": {
                "base_path": str(CE_BASE.relative_to(ROOT)),
                "path": "dev/plans/runs/0.8.25-slice-85/slice72-ce-manifest.json",
            },
            "global_rules": {
                "forbidden_output_patterns": ["[skip]", "0 tests", "timed out"],
                "retry_count": 0,
            },
            "sealed_thresholds": {
                "text_p50_ms_max": 20,
                "text_p99_ms_max": 150,
                "read_p50_ms_max": 80,
                "read_p99_ms_max": 300,
                "scale02_ack_ms_max": 1543.539,
                "scale02_total_ms_max": 1548.545,
                "ac013_total_ms_max": 1442.198,
                "write_spread_pct_max": 25,
                "ce_p95_ratio_max": 1.1,
            },
            "obligations": obligations,
        }

    def run_checker(
        self, value: dict, phase: str = "plan"
    ) -> subprocess.CompletedProcess[str]:
        path = self.repo / "manifest.json"
        path.write_text(json.dumps(value), encoding="utf-8")
        return subprocess.run(
            [
                "python3",
                str(CHECKER),
                "--manifest",
                str(path),
                "--repo",
                str(self.repo),
                "--phase",
                phase,
            ],
            check=False,
            capture_output=True,
            text=True,
        )

    def assert_rejected(self, value: dict, fragment: str, phase: str = "plan") -> None:
        result = self.run_checker(value, phase)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(fragment, result.stderr)

    @staticmethod
    def row(value: dict, obligation_id: str) -> dict:
        return next(row for row in value["obligations"] if row["id"] == obligation_id)

    def complete(self, value: dict) -> None:
        run_dir = self.repo / "dev/plans/runs/0.8.25-slice-85"
        run_dir.mkdir(parents=True, exist_ok=True)
        for row in value["obligations"]:
            if row["disposition"] == "unavailable":
                continue
            if row["id"] == "performance":
                row["disposition"] = "reuse"
                row["commands"] = []
                receipt = self.repo / "dev/plans/runs/slice80.json"
                receipt.write_text("retained pass\n", encoding="utf-8")
                row["retained_receipt"] = str(receipt.relative_to(self.repo))
                row["accepted_input_sha256"] = row["current_input_sha256"]
            log = run_dir / f"{row['id']}.log"
            log.write_text("PASS positive evidence\n", encoding="utf-8")
            row["evidence"] = [
                {
                    "path": str(log.relative_to(self.repo)),
                    "sha256": digest(log),
                    "tests": 200,
                    "skipped": 0,
                    "verdict": "pass",
                }
            ]
            row["verdict"] = "pass"
        for obligation_id in ("runtime-configuration", "linux-artifact-current-smoke"):
            self.row(value, obligation_id)["artifact_sha256"] = self.artifacts[
                "python-wheel"
                if obligation_id == "runtime-configuration"
                else "napi-linux-x64-gnu"
            ]["sha256"]

    def test_planning_manifest_validates(self) -> None:
        result = self.run_checker(self.manifest())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("30 obligations", result.stdout)

    def test_missing_or_duplicate_obligation_is_rejected(self) -> None:
        for mutation in ("missing", "duplicate"):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                if mutation == "missing":
                    value["obligations"].pop()
                else:
                    value["obligations"].append(copy.deepcopy(value["obligations"][0]))
                self.assert_rejected(value, "obligation IDs")

    def test_forbidden_disposition_is_rejected(self) -> None:
        value = self.manifest()
        self.row(value, "default-tree")["disposition"] = "waived"
        self.assert_rejected(value, "disposition")

    def test_final_pass_requires_positive_skip_free_evidence(self) -> None:
        for mutation, fragment in (
            ("missing", "evidence"),
            ("zero", "positive test count"),
            ("skip", "skipped must be zero"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                row = self.row(value, "default-tree")
                if mutation == "missing":
                    row["evidence"] = []
                elif mutation == "zero":
                    row["evidence"][0]["tests"] = 0
                else:
                    row["evidence"][0]["skipped"] = 1
                self.assert_rejected(value, fragment, "final")

    def test_stale_candidate_or_artifact_is_rejected(self) -> None:
        for mutation, fragment in (
            ("candidate", "candidate_sha"),
            ("artifact", "artifact_sha256"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                if mutation == "candidate":
                    self.row(value, "default-tree")["candidate_sha"] = "9" * 40
                else:
                    self.row(value, "runtime-configuration")["artifact_sha256"] = (
                        "9" * 64
                    )
                self.assert_rejected(value, fragment, "final")

    def test_relaxed_threshold_is_rejected(self) -> None:
        value = self.manifest()
        value["sealed_thresholds"]["text_p50_ms_max"] = 21
        self.assert_rejected(value, "sealed_thresholds")

    def test_false_reuse_is_rejected(self) -> None:
        value = self.manifest()
        row = self.row(value, "performance")
        row.update(
            disposition="reuse",
            commands=[],
            retained_receipt="dev/plans/runs/slice80.json",
            accepted_input_sha256="9" * 64,
        )
        self.assert_rejected(value, "reuse input digest")

    def test_final_rejects_missing_or_tampered_evidence_files(self) -> None:
        for mutation, fragment in (
            ("missing", "evidence file missing"),
            ("tampered", "evidence sha256 mismatch"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                path = (
                    self.repo / self.row(value, "default-tree")["evidence"][0]["path"]
                )
                if mutation == "missing":
                    path.unlink()
                else:
                    path.write_text("tampered\n", encoding="utf-8")
                self.assert_rejected(value, fragment, "final")

    def test_final_rejects_missing_reuse_receipt(self) -> None:
        value = self.manifest()
        self.complete(value)
        (self.repo / self.row(value, "performance")["retained_receipt"]).unlink()
        self.assert_rejected(value, "retained receipt file missing", "final")

    def test_final_rejects_input_digest_not_computed_from_paths(self) -> None:
        value = self.manifest()
        self.complete(value)
        self.row(value, "default-tree")["current_input_sha256"] = "8" * 64
        self.assert_rejected(value, "computed input digest", "final")

    def test_final_rejects_nonexistent_candidate_and_tampered_artifact(self) -> None:
        for mutation, fragment in (
            ("candidate", "candidate commit"),
            ("artifact", "artifact sha256 mismatch"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                if mutation == "candidate":
                    value["candidate"]["sha"] = "9" * 40
                    for row in value["obligations"]:
                        row["candidate_sha"] = "9" * 40
                    overlay = self.repo / value["ce_overlay"]["path"]
                    overlay_value = json.loads(overlay.read_text(encoding="utf-8"))
                    overlay_value["candidate_sha"] = "9" * 40
                    overlay.write_text(json.dumps(overlay_value), encoding="utf-8")
                else:
                    artifact = (
                        self.repo
                        / value["candidate"]["artifacts"]["python-wheel"]["path"]
                    )
                    artifact.write_text("tampered\n", encoding="utf-8")
                self.assert_rejected(value, fragment, "final")

    def test_final_rejects_unsealed_legacy_command_or_too_small_count(self) -> None:
        for mutation, fragment in (
            ("command", "legacy command contract"),
            ("count", "positive count contract"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                row = self.row(value, "default-tree")
                if mutation == "command":
                    row["commands"] = ["true"]
                else:
                    row["evidence"][0]["tests"] = 1
                self.assert_rejected(value, fragment, "final")

    def test_ac034c_is_the_only_unavailable_row_and_never_passes(self) -> None:
        for mutation in ("other-unavailable", "ac034c-pass"):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                if mutation == "other-unavailable":
                    self.row(value, "default-tree")["disposition"] = "unavailable"
                else:
                    self.row(value, "ac034c")["verdict"] = "pass"
                self.assert_rejected(value, "AC-034c")

    def test_ac034c_order_does_not_matter(self) -> None:
        value = self.manifest()
        value["obligations"].insert(0, value["obligations"].pop())
        result = self.run_checker(value)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_ce_overlay_may_only_change_candidate_sha(self) -> None:
        overlay_path = self.repo / self.manifest()["ce_overlay"]["path"]
        overlay = json.loads(overlay_path.read_text(encoding="utf-8"))
        overlay["steady_calls"] += 1
        overlay_path.write_text(json.dumps(overlay), encoding="utf-8")
        self.assert_rejected(self.manifest(), "CE overlay")


class Slice85InstalledRuntimeContractTest(unittest.TestCase):
    def test_smoke_runs_six_fresh_runtime_configuration_processes(self) -> None:
        text = SMOKE.read_text(encoding="utf-8")
        for marker in (
            "slice85-runtime-python-performance: pass",
            "slice85-runtime-python-conflict: pass",
            "slice85-runtime-python-invalid: pass",
            "slice85-runtime-node-performance: pass",
            "slice85-runtime-node-conflict: pass",
            "slice85-runtime-node-invalid: pass",
            "slice85-runtime-configuration-result: python=3 node=3 skipped=0",
        ):
            with self.subTest(marker=marker):
                self.assertIn(marker, text)


class Slice85NodeSupportContractTest(unittest.TestCase):
    def test_package_and_docs_support_node_24_and_25(self) -> None:
        package = json.loads((ROOT / "src/ts/package.json").read_text(encoding="utf-8"))
        self.assertEqual(package.get("engines", {}).get("node"), ">=24 <26")
        install = (ROOT / "docs/install/typescript.md").read_text(encoding="utf-8")
        compatibility = (ROOT / "docs/compatibility/index.md").read_text(
            encoding="utf-8"
        )
        readme = (ROOT / "src/ts/README.md").read_text(encoding="utf-8")
        for text in (install, compatibility, readme):
            self.assertIn("24.19.0", text)
            self.assertIn("25.9.0", text)

    def test_primary_ci_and_release_node_is_24(self) -> None:
        for relative in (".github/workflows/ci.yml", ".github/workflows/release.yml"):
            text = (ROOT / relative).read_text(encoding="utf-8")
            setup_count = text.count("uses: actions/setup-node@")
            self.assertGreater(setup_count, 0)
            self.assertEqual(text.count('node-version: "24.19.0"'), setup_count)
            self.assertNotIn('node-version: "25.9.0"', text)

    def test_node_24_arm64_archive_digest_is_sealed(self) -> None:
        expected = "01443c1e1a29e531ccad5a46fefa6df490d2189c49f7955904aecdbb0fe86fdc"
        for relative in (
            "scripts/release/Dockerfile.napi-manylinux",
            "scripts/release/napi-artifact-contract.sh",
        ):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("24.19.0", text)
            self.assertIn(expected, text)


if __name__ == "__main__":
    unittest.main()
