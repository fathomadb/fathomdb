"""Real short-child branch scheduling controls, never GPU engagement evidence."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_driver import run_program_step


class GBranchPollingTests(unittest.TestCase):
    def observe_short_child(self, stage, kind):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initial, ready = root / "initial", root / "ready"
            code = "\n".join([
                "import json,pathlib,sys,time",
                f"initial=pathlib.Path({str(initial)!r}); ready=pathlib.Path({str(ready)!r})",
                "deadline=time.monotonic()+3",
                "while not initial.exists() and time.monotonic()<deadline: time.sleep(0.001)",
                "assert initial.exists(); ready.touch(); time.sleep(0.4)",
                "out=pathlib.Path(sys.argv[2]); out.mkdir()",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            observations = []

            def observe(pid):
                # This is a scheduling witness, not a synthetic GPU observation.
                if not observations:
                    initial.touch()
                    live_ready = False
                else:
                    try:
                        state = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()[0]
                        live_ready = ready.exists() and state not in ("Z", "X")
                    except FileNotFoundError:
                        live_ready = False
                value = {"pid": pid, "live_ready": live_ready}
                observations.append(value)
                return value

            step = run_program_step([sys.executable, "-c", code],
                                    {"family": "G-O", "stage": stage, "kind": kind},
                                    root / "output", time.monotonic_ns() + 5 * 10**9,
                                    observe=observe)
            self.assertTrue(step["complete"])
            self.assertEqual(step["exit"], 0)
            self.assertEqual(step["gpu_samples"], observations)
            self.assertTrue(all(row["pid"] == step["pid"] for row in observations))
            return observations

    def test_branch_control_observes_ready_short_child(self):
        actual = self.observe_short_child("G-O", "branches")
        self.assertTrue(any(row["live_ready"] for row in actual),
                        "G-O branches missed the actual live ready short child")

    def test_natural_diagnostic_keeps_one_second_wait(self):
        actual = self.observe_short_child("G-O", "natural")
        self.assertEqual(len(actual), 1)
        self.assertFalse(actual[0]["live_ready"])

    def test_ordinary_stage_does_not_inherit_branch_wait(self):
        actual = self.observe_short_child("G-R", "branches")
        self.assertEqual(len(actual), 1)
        self.assertFalse(actual[0]["live_ready"])
