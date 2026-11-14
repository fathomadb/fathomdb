"""Actual retained growth stops the fixed calibration and cannot renew via resume."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_campaign
from test_g_closed_modes import references
from test_g_closed_plan import SPEC


class GClosedCalibrationStopTests(unittest.TestCase):
    def test_cap_consuming_first_child_retains_partial_and_stops_all_later_dispatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "calls"
            command = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sys",
                "c=json.loads(pathlib.Path(sys.argv[1]).read_text());o=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as f:f.write(c['id']+'\\n')",
                "(o/'retained-partial').write_bytes(b'x'*131072)",
                "(o/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])]
            spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-calibration")
            with patch.object(g_campaign, "CALIBRATION_CELL_BYTES", 65536):
                g_campaign.run_g_plan(spec, root / "results", command, allocation_root=root,
                                      reused=references())
                actual = json.loads((root / "results/calibration-outcomes.json").read_text())
                self.assertEqual(actual[0]["status"], "failed", "cell growth ignored its retained allowance")
                self.assertIn("calibration_cell_storage_exhausted", actual[0]["error"])
                self.assertEqual(calls.read_text().splitlines(), ["g-cal-engine-b10-s1"])
                self.assertTrue(all(row["status"].startswith("not_run_") for row in actual[1:]))
                self.assertEqual(Path(actual[0]["attempt"], "retained-partial").stat().st_size, 131072)
                g_campaign.run_g_plan(spec, root / "results", command, allocation_root=root, reused=references())
                g_campaign.run_g_plan(spec, root / "new-root", command, allocation_root=root, reused=references())
                self.assertEqual(calls.read_text().splitlines(), ["g-cal-engine-b10-s1"],
                                 "resume/new root retried a failed calibration or advanced past it")
                self.assertIsNone(json.loads((root / "g-budget.json").read_text())["inflight"])
