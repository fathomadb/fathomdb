"""A post-cleanup report stop cannot leave reusable phase credit."""

import json
import unittest

from m_runner import publish_selected
import test_m_selected_report
from test_m_calibration import LIMITS


class FinalizationCapTests(unittest.TestCase):
    def test_exhausted_final_publication_invalidates_the_original_selected_phase(self):
        fixture = test_m_selected_report.SelectedReportTests()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        guardian = fixture.root / "guardians" / "original"
        guardian.mkdir(parents=True)
        receipt = guardian / "guardian.json"
        receipt.write_text(json.dumps(fixture.fixture.guardian))
        ledger = fixture.fixture.base / "g-budget.json"
        ledger.write_text(json.dumps({"used_ns": 1000, "inflight": None,
            "selected_cell_usage": {fixture.cell["id"]: {"limits": LIMITS,
                "wall_ns": 900 * 10**9 - 500000000, "new_retained_bytes": 12345}}}))
        value = publish_selected(fixture.root)
        self.assertFalse(value["phase_qualified"])
        self.assertNotEqual(json.loads(receipt.read_text())["exit"], 0,
                            "exhausted final report left original phase reusable")


if __name__ == "__main__":
    unittest.main()
