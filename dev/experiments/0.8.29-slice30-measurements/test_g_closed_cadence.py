"""Real short-child observation cadence; samples are protocol controls, never GPU proof."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_driver import run_program_step


class GClosedCadenceTests(unittest.TestCase):
    def test_calibration_observes_real_short_child_between_initial_and_exit(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            command = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sys,time",
                "o=pathlib.Path(sys.argv[2]);o.mkdir();time.sleep(.24)",
                "(o/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])]
            observations = []
            def observe(pid):
                observations.append((pid, time.monotonic_ns()))
                return {"pid": pid, "protocol_only": True}
            step = run_program_step(command, {"family": "G-W", "stage": "G-W", "kind": "write", "calibration": True},
                                    root / "result", time.monotonic_ns() + 5 * 10**9, observe)
            self.assertTrue(step["complete"])
            self.assertGreaterEqual(len(observations), 3,
                                    "one-second cadence missed the real short calibration lifetime")
            self.assertTrue(all(pid == step["pid"] for pid, _ in observations))
            self.assertGreater(observations[-1][1], observations[0][1])
