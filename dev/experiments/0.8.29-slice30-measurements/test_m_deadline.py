"""Actual TERM-resistant child cleanup must fit inside the original absolute cap."""

from pathlib import Path
import sys
import tempfile
import time
import unittest

from m_attempt import run_attempt


class CleanupDeadlineTests(unittest.TestCase):
    """The work interval cannot consume time reserved for bounded owned cleanup."""

    def test_term_resistant_child_is_reaped_before_original_whole_attempt_deadline(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marker = root / "started"
            child = ("import pathlib,signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); "
                     f"pathlib.Path({str(marker)!r}).write_text('started'); time.sleep(60)")
            began = time.monotonic_ns()
            result = run_attempt({"id": "source-hung-child"}, root,
                                 [sys.executable, "-c", child], deadline_ns=began + 8 * 10**9)
            self.assertTrue(marker.exists(), "the actual resistant child must execute")
            self.assertLess(time.monotonic_ns() - began, 8 * 10**9,
                            "cleanup started after the absolute whole-attempt allowance")
            self.assertFalse(result["complete"])
            self.assertFalse(result["cleanup_uncertain"])
            self.assertEqual(result["live_owned_pids"], [])


if __name__ == "__main__":
    unittest.main()
