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
SHA = "1" * 40
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
        overlay["candidate_sha"] = SHA
        overlay_path = self.repo / "dev/plans/runs/0.8.25-slice-85/slice72-ce-manifest.json"
        overlay_path.parent.mkdir(parents=True)
        overlay_path.write_text(json.dumps(overlay), encoding="utf-8")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def obligation(self, obligation_id: str, origin: str = "legacy") -> dict:
        return {
            "id": obligation_id,
            "origin": origin,
            "disposition": "run",
            "candidate_sha": SHA,
            "current_input_sha256": HASH,
            "accepted_input_sha256": None,
            "commands": ["true"],
            "retained_receipt": None,
            "artifact_sha256": None,
            "evidence": [],
            "verdict": "pending",
        }

    def manifest(self) -> dict:
        legacy = json.loads(LEGACY.read_text(encoding="utf-8"))
        obligations = [self.obligation(cell["id"]) for cell in legacy["cells"]]
        obligations.extend(
            self.obligation(obligation_id, "additional")
            for obligation_id in ("runtime-configuration", "protected-writes", "slice72-ce")
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
                "sha": SHA,
                "tree_sha256": HASH,
                "artifacts": {
                    "python-wheel": HASH,
                    "napi-linux-x64-gnu": HASH,
                    "cli-linux-x64-gnu": HASH,
                },
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

    def run_checker(self, value: dict, phase: str = "plan") -> subprocess.CompletedProcess[str]:
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
        for row in value["obligations"]:
            if row["disposition"] == "unavailable":
                continue
            if row["id"] == "performance":
                row["disposition"] = "reuse"
                row["commands"] = []
                row["retained_receipt"] = "dev/plans/runs/slice80.json"
                row["accepted_input_sha256"] = row["current_input_sha256"]
            row["evidence"] = [
                {
                    "path": f"logs/{row['id']}.log",
                    "sha256": HASH,
                    "tests": 1,
                    "skipped": 0,
                    "verdict": "pass",
                }
            ]
            row["verdict"] = "pass"
        for obligation_id in ("runtime-configuration", "linux-artifact-current-smoke"):
            self.row(value, obligation_id)["artifact_sha256"] = HASH

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
        for mutation, fragment in (("candidate", "candidate_sha"), ("artifact", "artifact_sha256")):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                if mutation == "candidate":
                    self.row(value, "default-tree")["candidate_sha"] = "9" * 40
                else:
                    self.row(value, "runtime-configuration")["artifact_sha256"] = "9" * 64
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

    def test_ac034c_is_the_only_unavailable_row_and_never_passes(self) -> None:
        for mutation in ("other-unavailable", "ac034c-pass"):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                if mutation == "other-unavailable":
                    self.row(value, "default-tree")["disposition"] = "unavailable"
                else:
                    self.row(value, "ac034c")["verdict"] = "pass"
                self.assert_rejected(value, "AC-034c")

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
        compatibility = (ROOT / "docs/compatibility/index.md").read_text(encoding="utf-8")
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


if __name__ == "__main__":
    unittest.main()
