"""Explicit live D CLI/controller connection with harmless owned payloads only."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class DPublicEntryTests(unittest.TestCase):
    def test_public_source_entry_owns_initial_four_and_publishes_all24(self):
        task = Path(os.environ["S30_TASK"])
        with tempfile.TemporaryDirectory(prefix="d-public-entry-", dir=task/"tmp") as temporary:
            root = Path(temporary)/"source"
            result = subprocess.run([sys.executable, str(Path(__file__).with_name("d_runner.py")),
                "source-control", "--root", str(root)], text=True,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30)
            report_path = root/"d-report.json"
            self.assertTrue(report_path.exists(),
                "public D CLI never entered its guarded fixed24 controller: "+result.stdout)
            report = json.loads(report_path.read_text())
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertEqual(report["scope"], "isolated-harmless-source-control")
            self.assertFalse(report["qualifies_natural"])
            self.assertTrue(report["source_control_complete"])
            expected = ["d-c-held-n10000-r0", "d-t-held-n10000-r0",
                        "d-t-held-n10000-r1", "d-c-held-n10000-r1"]
            self.assertEqual((root/"dispatches.jsonl").read_text().splitlines(), expected)
            rows = report["rows"]
            self.assertEqual(len(rows), 24)
            self.assertEqual([row["id"] for row in rows[:4]], expected)
            self.assertTrue(all(row["status"] == "failed" for row in rows[:4]))
            self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))
            guardian = report["guardian"]
            self.assertFalse(guardian["cleanup_uncertain"])
            self.assertTrue(guardian["boundary"]["removed_exact_empty_boundary"])
            budget = json.loads((root/"g-budget.json").read_text())
            self.assertEqual(budget["cell_used_ns"]["g-prior"], 101)
            self.assertEqual(budget["common_used_ns"]["M"], 17)
            self.assertEqual(budget["used_ns"]-123,
                sum(budget["cell_used_ns"][cell] for cell in expected)+budget["common_used_ns"]["D"])
            self.assertIsNone(budget["inflight"])
            self.assertIsNone(budget["d_guardian"])
            self.assertTrue(all(row["native"]["unit_diagnostic"] for row in rows[:4]))
            self.assertEqual(set(json.loads((root/"d-state.json").read_text())), set(expected))


if __name__ == "__main__":
    unittest.main()
