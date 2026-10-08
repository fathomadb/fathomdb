"""The receipt must account for the exact live operation set."""

import json
import subprocess
import sys
import unittest

from audit import ROOT, audit, check_state, read_state


class OperationAccountingTest(unittest.TestCase):
    def test_external_receipt_accounts_for_each_live_operation(self):
        audit(json.loads((ROOT / "operations.json").read_text()))

    def test_negative_control_missing_operation_is_rejected(self):
        rows = json.loads((ROOT / "operations.json").read_text())
        with self.assertRaisesRegex(ValueError, "canonical live set"):
            audit(rows[:-1])

    def test_negative_control_retained_graph_edge_is_rejected(self):
        state = read_state()
        check_state(state)
        with self.assertRaisesRegex(ValueError, "persisted state differs"):
            check_state({**state, "graph_edges": 1})

    def test_retained_receipt_replays_after_integration(self):
        completed = subprocess.run(
            [sys.executable, str(ROOT / "audit.py")],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(json.loads(completed.stdout)["counts"]["executed"], 42)


if __name__ == "__main__":
    unittest.main()
