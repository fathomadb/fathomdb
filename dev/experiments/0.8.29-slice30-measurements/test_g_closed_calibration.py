"""Four fixed calibration controls are separate from required primary measurements."""

import json
from pathlib import Path
import tempfile
import unittest

from g_campaign import run_g_plan
from test_g_closed_modes import harmless, references
from test_g_closed_plan import SPEC


class GClosedCalibrationTests(unittest.TestCase):
    def test_four_calibrations_stop_with_all_primary_obligations_retained(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "calls"
            spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-calibration")
            rows = run_g_plan(spec, root / "results", harmless(calls), allocation_root=root,
                              reused=references())
            expected = ["g-cal-engine-b10-s1", "g-cal-sdk-b10-s1",
                        "g-cal-engine-b1-s100", "g-cal-sdk-b1-s100"]
            self.assertEqual(calls.read_text().splitlines(), expected,
                             "calibration escaped its fixed four diagnostic subjects")
            self.assertEqual(len(rows), 84)
            self.assertEqual(sum(row["status"] == "reused_complete" for row in rows), 11)
            self.assertEqual(sum(row["status"] == "not_run_stage" for row in rows), 73)
            calibration = json.loads((root / "results/calibration-outcomes.json").read_text())
            self.assertEqual(len(calibration), 4)
            self.assertTrue(all(row["status"] == "complete" for row in calibration))
            self.assertEqual([(r["cell"]["adapters"], r["cell"]["batch_size"],
                               r["cell"]["searches_per_batch"]) for r in calibration],
                             [(["engine"], 10, 1), (["sdk"], 10, 1),
                              (["engine"], 1, 100), (["sdk"], 1, 100)])
            self.assertTrue(all(r["cell"]["routes"] == ["text"] and r["cell"]["warmup"] == 500
                                and r["cell"]["samples"] == 1000 for r in calibration))
            run_g_plan(spec, root / "results", harmless(calls), allocation_root=root,
                       reused=references())
            self.assertEqual(calls.read_text().splitlines(), expected,
                             "resume replayed completed calibration or advanced to primary work")
