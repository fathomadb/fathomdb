#!/usr/bin/env python3
"""Mutation tests for the Slice 75 closure-manifest verifier."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "scripts/release/verify-slice75-closure.py"
MANIFEST = ROOT / "dev/plans/0.8.25/features/slice-75/slice75-closure-manifest.json"


class Slice75ClosureManifestTest(unittest.TestCase):
    def manifest(self) -> dict:
        return json.loads(MANIFEST.read_text(encoding="utf-8"))

    def run_checker(self, value: dict) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(prefix="slice75-manifest-test-") as directory:
            path = Path(directory) / "manifest.json"
            path.write_text(json.dumps(value), encoding="utf-8")
            return subprocess.run(
                ["python3", str(CHECKER), "--manifest", str(path), "--repo", str(ROOT)],
                check=False,
                capture_output=True,
                text=True,
            )

    def assert_rejected(self, value: dict, fragment: str) -> None:
        result = self.run_checker(value)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(fragment, result.stderr)

    def cell(self, manifest: dict, cell_id: str) -> dict:
        return next(cell for cell in manifest["cells"] if cell["id"] == cell_id)

    def test_approved_manifest_validates(self) -> None:
        result = self.run_checker(self.manifest())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("26 cells", result.stdout)

    def test_missing_required_cell_is_rejected(self) -> None:
        value = self.manifest()
        value["cells"] = [cell for cell in value["cells"] if cell["id"] != "jetson-tegra"]
        self.assert_rejected(value, "required cells")

    def test_relaxed_performance_limit_is_rejected(self) -> None:
        value = self.manifest()
        self.cell(value, "performance")["expected"]["vector_p50_ms_max"] = 81
        self.assert_rejected(value, "vector_p50_ms_max")

    def test_zero_or_wrong_expected_count_is_rejected(self) -> None:
        value = self.manifest()
        self.cell(value, "model-python")["expected"]["passed"] = 0
        self.assert_rejected(value, "model-python")

    def test_skip_guard_cannot_be_removed(self) -> None:
        value = self.manifest()
        value["global_rules"]["forbidden_output_patterns"].remove("[skip]")
        self.assert_rejected(value, "forbidden_output_patterns")

    def test_network_environment_must_be_explicitly_unset(self) -> None:
        value = self.manifest()
        del self.cell(value, "model-engine")["unset_environment"]
        self.assert_rejected(value, "FATHOMDB_SKIP_NETWORK_TESTS")

    def test_stale_retained_receipt_digest_is_rejected(self) -> None:
        value = self.manifest()
        value["retained"][0]["receipt_sha256"] = "0" * 64
        self.assert_rejected(value, "receipt_sha256")

    def test_source_fallback_cannot_be_enabled(self) -> None:
        value = self.manifest()
        self.cell(value, "linux-artifact-current-smoke")["expected"]["source_fallback"] = True
        self.assert_rejected(value, "source_fallback")

    def test_data_plane_witness_cannot_claim_answer_quality(self) -> None:
        value = self.manifest()
        self.cell(value, "global-01-native")["expected"]["answer_quality"] = True
        self.assert_rejected(value, "answer_quality")

    def test_unknown_top_level_key_is_rejected(self) -> None:
        value = copy.deepcopy(self.manifest())
        value["unreviewed"] = True
        self.assert_rejected(value, "unknown top-level")


if __name__ == "__main__":
    unittest.main()
