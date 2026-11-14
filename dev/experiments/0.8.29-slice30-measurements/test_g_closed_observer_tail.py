"""Actual maximum callback tail bounds process-exit observation latency, never GPU work."""

from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_driver import run_program_step


class GClosedObserverTailTests(unittest.TestCase):
    def test_actual_longest_callback_is_retained_instead_of_average(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            command = [sys.executable, "-c", "import json,pathlib,sys,time;o=pathlib.Path(sys.argv[2]);o.mkdir();time.sleep(.27);(o/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True)))"]
            calls = []
            def observe(pid):
                delay = .06 if not calls else .005
                time.sleep(delay)
                calls.append(pid)
                return {"pid": pid, "protocol_only": True}
            step = run_program_step(command, {"family": "G-W", "stage": "G-W", "calibration": True},
                                    root / "result", time.monotonic_ns()+5*10**9, observe)
            self.assertTrue(step["complete"])
            self.assertGreaterEqual(step.get("observation_max_wall_ns", 0), 60_000_000,
                                    "actual longest callback tail is unavailable")
            self.assertLessEqual(step["observation_max_wall_ns"], step["observation_wall_ns"])
            self.assertGreater(step["observation_max_wall_ns"],
                               step["observation_wall_ns"] / step["observation_count"])
