"""Explicit owned live controller integration; never a natural MCP campaign."""

import json
from pathlib import Path
import subprocess
import sys
import unittest

from m_native_tests import NativeCase


class PublicDispatchTests(NativeCase):
    """The committed public path must include real preparation, cell and common charges."""

    def test_public_source_dispatch_accounts_actual_normal_cli_cells_and_common_wall(self):
        root = self.root / "dispatch"
        child = subprocess.run([sys.executable, str(Path(__file__).with_name("m_runner.py")),
            "source-dispatch", "--root", str(root), "--cli", str(self.cli), "--helper", str(self.helper)],
            capture_output=True, timeout=45)
        self.assertEqual(child.returncode, 0, child.stderr.decode())
        result = json.loads(child.stdout)
        self.assertTrue(result["complete"])
        ledger = json.loads((root / "g-budget.json").read_text())
        self.assertGreater(ledger["used_ns"], 0, "public dispatch dropped all actual source-control wall")
        self.assertGreater(ledger["common_used_ns"]["M"], 0)
        self.assertEqual(set(ledger["cell_used_ns"]), {"M0-text", "M0-structured"})
        self.assertEqual(ledger["used_ns"], sum(ledger["cell_used_ns"].values())
                         + ledger["common_used_ns"]["M"])
        self.assertIsNone(ledger["inflight"])
        self.assertIsNone(ledger["m_guardian"])
        self.assertEqual([row["id"] for row in result["rows"]], ["M0-text", "M0-structured"])
        self.assertFalse(result["qualifies_natural"])

    def test_public_source_refusal_keeps_required_rows_without_native_preparation(self):
        root = self.root / "refused"
        child = subprocess.run([sys.executable, str(Path(__file__).with_name("m_runner.py")),
            "source-dispatch", "--root", str(root), "--cli", str(self.cli), "--helper", str(self.helper),
            "--source-total-ns", "1"], capture_output=True, timeout=45)
        self.assertEqual(child.returncode, 1, "infeasible public selection executed actual MCP work")
        result = json.loads(child.stdout)
        self.assertEqual([row["id"] for row in result["rows"]], ["M0-text", "M0-structured"])
        self.assertTrue(all(row["status"] == "held" for row in result["rows"]))
        self.assertFalse(list(root.rglob("*.sqlite")))
        self.assertEqual(json.loads((root / "g-budget.json").read_text())["used_ns"], 0)


if __name__ == "__main__":
    unittest.main()
