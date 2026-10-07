"""The receipt must account for the exact live operation set."""

import json
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


if __name__ == "__main__":
    unittest.main()
