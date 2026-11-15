"""Admission work consumes the same absolute attempt deadline."""

from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_attempt import run_g_attempt


class GSerialDeadlineTests(unittest.TestCase):
    def test_final_admission_cannot_launch_after_consuming_dispatch_allowance(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            def admit():
                time.sleep(.15)
            outcome = run_g_attempt({"id": "deadline-control"}, root,
                [sys.executable, "-c", "pass"], time.monotonic_ns()+100_000_000, admit=admit)
            self.assertIsNone(outcome["pid"], "final admission consumed the deadline but still created a child")
            self.assertEqual(outcome["error"], "cell timeout before child start")
