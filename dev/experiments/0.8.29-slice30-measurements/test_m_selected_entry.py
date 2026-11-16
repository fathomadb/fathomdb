"""Selected required-cell dispatch and real harmless-child accounting."""

from contextlib import ExitStack
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from m_calibration import preparation_bound
from m_guard import guard
from m_runner import main
from test_m_calibration import LIMITS
from test_m_calibration_guard import FakeBoundary


class SelectedEntryTests(unittest.TestCase):
    def setUp(self):
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        self.root = Path(self.stack.enter_context(tempfile.TemporaryDirectory()))
        self.ledger = self.root / "g-budget.json"
        self.ledger.write_text(json.dumps({"schema": "slice30-gd-budget/v1", "used_ns": 123,
                                          "inflight": None, "cell_used_ns": {}}))
        self.profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
            "ledger": str(self.ledger), "whole_cell_ns": 900 * 10**9,
            "total_shared_ns": 14400 * 10**9, "total_shared_retained_bytes": 16 * 1024**3,
            "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0,
            "other_work_cost_status": "HELD_UNAVAILABLE", "reservation_disposition": "one required M cell"}
        self.stack.enter_context(patch("m_guard.TaskBoundary", FakeBoundary))
        self.stack.enter_context(patch("m_guard.resource_sample", return_value={
            "root_free": 1000 * 1024**3, "task_free": 1000 * 1024**3,
            "memory_available": 100 * 1024**3, "owned": 0}))

    def test_guardian_charges_actual_selected_cell_phase_without_calibration_credit(self):
        result = guard([sys.executable, "-c", "pass"], allocation=self.root, profile=self.profile,
            root=self.root / "selected", task=self.root,
            cell_allowance={"cell_id": "M-R-1-1-debug", "limits": LIMITS})
        self.assertEqual(result["exit"], 0)
        value = json.loads(self.ledger.read_text())
        self.assertIn("selected_cell_usage", value, "selected phase lacks cumulative accounting")
        self.assertEqual(value["selected_cell_usage"]["M-R-1-1-debug"]["wall_ns"], value["used_ns"] - 123)
        self.assertNotIn("calibration_usage", value)
        self.assertIsNone(value["m_guardian"])

    def test_public_run_selects_exact_canonical_cell_and_original_allowance(self):
        profile = self.root / "profile.json"
        profile.write_text(json.dumps(self.profile))
        options = self.root / "options.json"
        options.write_text(json.dumps({"allocation": str(self.root), "profile": str(profile),
                                       "cell_allowance": LIMITS}))
        argv = ["m_runner.py", "run", str(self.root / "selected"), "--options", str(options),
                "--cell", "M-R-1-1-debug"]
        with patch.dict(os.environ, AGENT_LONG="1", S30_TASK=str(self.root)), patch("sys.argv", argv), \
                patch("m_guard.guard", return_value={"exit": 0, "cleanup_uncertain": False}) as guarded:
            self.assertEqual(main(), 0, "public selected required cell was not dispatched")
        self.assertEqual(guarded.call_count, 1)
        args, kwargs = guarded.call_args
        self.assertIn("_selected", args[0])
        self.assertIn("M-R-1-1-debug", args[0])
        self.assertEqual(kwargs["cell_allowance"], {"cell_id": "M-R-1-1-debug", "limits": LIMITS})
        self.assertEqual(kwargs["profile"], self.profile)

    def test_selected_bound_uses_enforced_capture_and_complete_remaining_terms(self):
        queries = self.root / "queries.jsonl"
        queries.write_text('"alpha"\n' * 500)
        manifest = {"scope": "registered-M-selected-cell", "allocation": str(self.root),
            "inputs": {"queries": {"path": str(queries)}}, "package": [{"bytes": 715223040}],
            "plan": [{"family": "M-R", "copies": 1, "config": {"idle_timeout_secs": 60}}]}
        bound = preparation_bound(manifest)
        self.assertGreaterEqual(bound, 715223040 + 6 * 67108864 + 3 * 8388608)
        self.assertLessEqual(bound, 1382159668,
                             "the complete enforced producer bound still cannot fund this continuation")

    def test_alternating_preparation_reserves_two_complete_packages(self):
        queries = self.root / "queries.jsonl"
        queries.write_text('"alpha"\n' * 500)
        base = {"inputs": {"queries": {"path": str(queries)}}, "package": [{"bytes": 715223040}]}
        one = preparation_bound(dict(base, plan=[{"copies": 1}]))
        two = preparation_bound(dict(base, plan=[{"copies": 2}]))
        self.assertEqual(two - one, 715223040, "A/B/A second package was not reserved")


if __name__ == "__main__":
    unittest.main()
