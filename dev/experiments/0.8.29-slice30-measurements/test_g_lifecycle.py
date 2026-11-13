"""Owned harmless-process lifecycle controls, without product/native execution."""

import json
import os
from pathlib import Path
import sys
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

from g_attempt import run_g_attempt, stop_owned_group


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


class GSignalPinTests(unittest.TestCase):
    def test_actual_leader_stays_unreaped_through_every_owned_group_signal(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ready = root / "leader-started"
            code = (f"import pathlib,time; pathlib.Path({str(ready)!r}).write_text('ready'); "
                    "time.sleep(2)")
            child = subprocess.Popen([sys.executable, "-c", code], start_new_session=True)
            until = time.monotonic() + 2
            while not ready.exists() and time.monotonic() < until:
                time.sleep(0.005)
            self.assertTrue(ready.exists(), "actual group leader did not start")
            observed = []
            actual_killpg = os.killpg
            def only_forward_while_pinned(group, stop_signal):
                self.assertEqual(group, child.pid)
                try:
                    fields = Path(f"/proc/{child.pid}/stat").read_text().split(") ", 1)[1].split()
                    facts = {"state": fields[0], "pgid": int(fields[2]), "sid": int(fields[3])}
                except FileNotFoundError:
                    facts = None
                observed.append(facts)
                if facts is None:
                    raise ProcessLookupError("suppressed unsafe signal attempt after ownership pin disappeared")
                self.assertEqual((facts["pgid"], facts["sid"]), (child.pid, child.pid))
                return actual_killpg(group, stop_signal)
            with patch("g_attempt.os.killpg", side_effect=only_forward_while_pinned):
                stop_owned_group(child)
            self.assertGreaterEqual(len(observed), 1)
            self.assertTrue(all(facts is not None for facts in observed),
                            "owned group signalling attempted after actual leader pin was reaped")
            self.assertIsNotNone(child.returncode)

class GSuccessfulLeaderTests(unittest.TestCase):
    def test_successful_leader_cannot_complete_with_actual_live_owned_descendant(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ready, late = root / "descendant-started", root / "descendant-late"
            descendant = "\n".join([
                "import json,os,pathlib,time",
                f"pathlib.Path({str(ready)!r}).write_text(json.dumps({{'pid':os.getpid(),'pgid':os.getpgrp()}}))",
                "time.sleep(0.2)",
                f"pathlib.Path({str(late)!r}).write_text('survived successful leader')",
            ])
            code = "\n".join([
                "import json,pathlib,subprocess,sys,time",
                f"subprocess.Popen([sys.executable,'-c',{descendant!r}])",
                "until=time.monotonic()+2",
                f"while not pathlib.Path({str(ready)!r}).exists() and time.monotonic()<until: time.sleep(0.005)",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            outcome = run_g_attempt({"id": "successful-leader"}, root,
                                    [sys.executable, "-c", code], time.monotonic_ns() + 5 * 10**9)
            time.sleep(0.35)
            self.assertTrue(ready.exists(), "actual owned descendant was not established")
            facts = json.loads(ready.read_text())
            self.assertEqual(facts["pgid"], outcome["pid"])
            self.assertNotEqual(facts["pid"], outcome["pid"])
            self.assertEqual(outcome["child_exit"], 0)
            self.assertEqual(outcome["status"], "failed", "successful leader was accepted with surviving owned work")
            self.assertEqual(outcome["error"], "owned_work_survived_leader")
            self.assertFalse(late.exists(), "actual descendant continued after successful leader completion")
            self.assertFalse(outcome["cleanup_uncertain"])
            self.assertEqual(outcome["live_owned_pids"], [])
