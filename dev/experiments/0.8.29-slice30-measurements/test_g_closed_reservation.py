"""Fixed calibration admission reserves all remaining cell/common allowances first."""

import json
from pathlib import Path
import tempfile
import unittest

from g_campaign import run_g_plan
from test_g_closed_modes import harmless, references
from test_g_closed_plan import SPEC


class GClosedReservationTests(unittest.TestCase):
    def test_remaining_phase_worst_case_refuses_before_first_copy_or_program(self):
        with tempfile.TemporaryDirectory() as temporary:
            allocation = Path(temporary)
            token = {"consumed": True, "original": "retained"}
            (allocation / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 8000*10**9, "inflight": None, "cell_used_ns": {},
                "exploratory_seed100k": token}))
            spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-calibration")
            rows = run_g_plan(spec, allocation / "results", harmless(allocation / "forbidden"),
                              allocation_root=allocation, reused=references())
            self.assertFalse((allocation / "forbidden").exists(), "calibration dispatched without funding6000s+600s remaining")
            actual = json.loads((allocation / "results/calibration-outcomes.json").read_text())
            self.assertEqual(len(actual), 4)
            self.assertTrue(all(row["status"] == "not_run_cap" for row in actual))
            self.assertTrue(all(row["reason"] == "calibration_phase_reservation" for row in actual))
            self.assertEqual(actual[0]["reservation"]["remaining_time_ns"], 6600*10**9)
            self.assertEqual(actual[0]["reservation"]["new_bytes"], 13*1024**3)
            self.assertEqual(len(rows), 84)
            self.assertEqual(sum(row["status"] == "reused_complete" for row in rows), 11)
            budget = json.loads((allocation / "g-budget.json").read_text())
            self.assertEqual(budget["used_ns"], 8000*10**9)
            self.assertEqual(budget["exploratory_seed100k"], token)
            self.assertIsNone(budget["inflight"])
