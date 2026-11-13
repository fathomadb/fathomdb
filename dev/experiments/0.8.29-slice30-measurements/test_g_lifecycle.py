"""Owned harmless-process lifecycle controls, without product/native execution."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_attempt import run_g_attempt


class GMonitorExceptionTests(unittest.TestCase):
    def test_monitor_exception_stops_actual_child_and_retains_failed_attempt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ready, late = root / "child-started", root / "late-write"
            code = "\n".join([
                "import os,pathlib,time",
                f"pathlib.Path({str(ready)!r}).write_text(str(os.getpid()))",
                "time.sleep(0.2)",
                f"pathlib.Path({str(late)!r}).write_text('escaped monitor exception')",
            ])
            def monitor(pid):
                until = time.monotonic() + 2
                while not ready.exists() and time.monotonic() < until:
                    time.sleep(0.005)
                self.assertTrue(ready.exists(), "actual child did not reach the monitor control")
                self.assertEqual(int(ready.read_text()), pid)
                raise RuntimeError("controlled monitor exception after actual child start")
            outcome = None
            try:
                outcome = run_g_attempt({"id": "monitor-exception"}, root,
                                        [sys.executable, "-c", code], time.monotonic_ns() + 5 * 10**9,
                                        monitor=monitor)
            except RuntimeError:
                pass
            time.sleep(0.35)
            self.assertTrue(ready.exists())
            self.assertFalse(late.exists(), "actual owned child continued after monitor exception")
            self.assertIsNotNone(outcome, "monitor exception escaped without a retained failed outcome")
            self.assertEqual(outcome["status"], "failed")
            self.assertEqual(outcome["error"],
                             "RuntimeError: controlled monitor exception after actual child start")
            self.assertIsNotNone(outcome["child_exit"])
            self.assertEqual(json.loads((Path(outcome["attempt"]) / "attempt.json").read_text()), outcome)
            self.assertIn("controlled monitor exception", (Path(outcome["attempt"]) / "child.log").read_text())


if __name__ == "__main__":
    unittest.main()
