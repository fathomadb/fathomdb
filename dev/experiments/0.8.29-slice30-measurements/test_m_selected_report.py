"""Final M publication needs original cleanup and supports per-cell build variants."""

import json
from pathlib import Path
import unittest

from m_controller import load_plan
from m_runner import publish_selected, report, reused_rows
import test_m_reuse
from test_m_calibration import LIMITS


class SelectedReportTests(unittest.TestCase):
    def setUp(self):
        self.fixture = test_m_reuse.ReuseTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root = self.fixture.base / "current"
        self.root.mkdir()
        self.cell = load_plan()[3]
        self.manifest = dict(self.fixture.manifest, scope="registered-M-selected-cell",
            uuid="new-freeze", source_sha="new-tools", plan=[self.cell], reuse_from=[],
            cell_allowance=LIMITS, profile={"total_shared_ns": 14400 * 10**9,
                "total_shared_retained_bytes": 16 * 1024**3, "reserved_other_work_ns": 0})
        (self.root / "manifest.json").write_text(json.dumps(self.manifest))

    def test_controller_report_is_explicitly_provisional_before_final_guardian(self):
        value = report(self.root)
        self.assertIn("phase_qualified", value, "selected report lacks final cleanup qualification")
        self.assertFalse(value["phase_qualified"])
        self.assertFalse(value["complete"])

    def test_final_publication_charges_original_selected_allowance(self):
        guardian = self.root / "guardians" / "original"
        guardian.mkdir(parents=True)
        (guardian / "guardian.json").write_text(json.dumps(self.fixture.guardian))
        ledger = self.fixture.base / "g-budget.json"
        ledger.write_text(json.dumps({"used_ns": 1000, "inflight": None,
            "selected_cell_usage": {self.cell["id"]: {"limits": LIMITS, "wall_ns": 500,
                                                      "new_retained_bytes": 12345}}}))
        value = publish_selected(self.root)
        self.assertIsInstance(value, dict, "final selected report was not published")
        self.assertTrue(value["phase_qualified"])
        final = json.loads(ledger.read_text())
        self.assertGreater(final["used_ns"], 1000)
        self.assertEqual(final["selected_cell_usage"][self.cell["id"]]["new_retained_bytes"], 12345)

    def test_completed_providerfree_row_reuses_its_own_build_without_rebuilding_all_variants(self):
        original_path = self.fixture.old / "manifest.json"
        original = json.loads(original_path.read_text())
        cell = load_plan()[0]
        build = dict(original["builds"]["ordinary"], source_sha="actual-providerfree-native")
        original.update(scope="registered-M-selected-cell", plan=[cell], builds={"providerfree": build})
        original_path.write_text(json.dumps(original))
        attempt = self.fixture.old / "results" / cell["id"] / "attempt-0001"
        attempt.mkdir(parents=True)
        outcome = {"id": cell["id"], "cell": cell, "status": "complete", "complete": True,
                   "exit": 0, "cleanup_uncertain": False, "live_owned_pids": [], "attempt": str(attempt)}
        (self.fixture.old / "results/campaign-state.json").write_text(json.dumps([outcome]))
        (attempt / "result.json").write_text(json.dumps({"complete": True, "cell": cell,
            "freeze_uuid": original["uuid"], "selected_build": build, "gpu": {"complete": True}}))
        current = dict(self.fixture.manifest, reuse_from=[{"root": str(self.fixture.old), "cell_id": cell["id"]}])
        try:
            value = reused_rows(current)
        except KeyError:
            value = {}
        self.assertIn(cell["id"], value, "prior providerfree evidence wrongly requires a current providerfree rebuild")


if __name__ == "__main__":
    unittest.main()
