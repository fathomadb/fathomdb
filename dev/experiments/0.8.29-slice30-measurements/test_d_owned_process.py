"""Real harmless-child D dispatch/cleanup controls; no database or scientific run."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from d_attempt import run_owned_cell


class DOwnedProcessTests(unittest.TestCase):
    def test_actual_success_retains_native_result_and_positive_observed_cost(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ready = root / "started"
            code = "\n".join([
                "import json,pathlib,sys",
                f"pathlib.Path({str(ready)!r}).write_text('started')",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True,'unit_diagnostic':True}))",
            ])
            deadline = time.monotonic_ns() + 10*10**9
            row = run_owned_cell({"id": "owned-success"}, root,
                                 [sys.executable, "-c", code], deadline)
            self.assertTrue(ready.exists(), "D dispatch never launched its owned child")
            self.assertEqual(row["status"], "complete")
            self.assertEqual(row["native"], {"complete": True, "unit_diagnostic": True})
            self.assertEqual(row["child_exit"], 0)
            self.assertFalse(row["cleanup_uncertain"])
            self.assertEqual(row["live_owned_pids"], [])
            self.assertEqual(row["deadline_ns"], deadline)
            self.assertLessEqual(row["child_deadline_ns"], deadline - 6*10**9)
            self.assertGreater(row["elapsed_ns"], 0)
            self.assertGreater(row["peak_bytes"], 0)
            self.assertEqual(row["attempt_id"], "attempt-0001")
            self.assertEqual(json.loads((Path(row["attempt"])/"attempt.json").read_text()), row)

    def test_deadline_stops_actual_work_inside_outer_cleanup_allowance(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ready, late = root / "started", root / "late-write"
            code = "\n".join([
                "import pathlib,time",
                f"pathlib.Path({str(ready)!r}).write_text('started')",
                "time.sleep(0.6)",
                f"pathlib.Path({str(late)!r}).write_text('escaped deadline')",
            ])
            deadline = time.monotonic_ns() + 6_200_000_000
            row = run_owned_cell({"id": "owned-deadline"}, root,
                                 [sys.executable, "-c", code], deadline)
            time.sleep(0.7)
            self.assertTrue(ready.exists(), "deadline control never reached actual child work")
            self.assertFalse(late.exists(), "D owned work survived its deadline cleanup")
            self.assertEqual(row["status"], "failed")
            self.assertIn("timeout", row["error"])
            self.assertFalse(row["cleanup_uncertain"])
            self.assertEqual(row["live_owned_pids"], [])
            self.assertEqual(row["deadline_ns"], deadline)
            self.assertLessEqual(row["child_deadline_ns"], deadline - 6*10**9)
            self.assertGreater(row["elapsed_ns"], 0)
            self.assertGreater(row["peak_bytes"], 0)
            self.assertIsNone(row["native"])

    def test_successful_leader_cannot_qualify_with_live_owned_descendant(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ready, late = root / "descendant-started", root / "late-write"
            descendant = "\n".join([
                "import pathlib,time",
                f"pathlib.Path({str(ready)!r}).write_text('started')",
                "time.sleep(0.6)",
                f"pathlib.Path({str(late)!r}).write_text('escaped successful leader')",
            ])
            code = "\n".join([
                "import json,pathlib,subprocess,sys,time",
                f"subprocess.Popen([sys.executable,'-c',{descendant!r}])",
                "until=time.monotonic()+2",
                f"while not pathlib.Path({str(ready)!r}).exists() and time.monotonic()<until: time.sleep(0.005)",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True,'unit_diagnostic':True}))",
            ])
            row = run_owned_cell({"id": "owned-descendant"}, root,
                                 [sys.executable, "-c", code], time.monotonic_ns() + 10*10**9)
            time.sleep(0.7)
            self.assertTrue(ready.exists(), "owned descendant was never established")
            self.assertFalse(late.exists(), "D accepted a successful leader with surviving work")
            self.assertEqual(row["child_exit"], 0)
            self.assertEqual(row["status"], "failed")
            self.assertEqual(row["error"], "owned_work_survived_leader")
            self.assertFalse(row["cleanup_uncertain"])
            self.assertEqual(row["live_owned_pids"], [])


if __name__ == "__main__":
    unittest.main()
