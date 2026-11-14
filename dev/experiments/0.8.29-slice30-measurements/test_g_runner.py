"""Bounded production-runner contracts using actual harmless command controls."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_campaign import make_g_plan
from g_runner import execute_cell


class GProductionQualificationTests(unittest.TestCase):
    def test_protocol_only_child_reports_cannot_qualify_positive_default_g0(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        cell = make_g_plan(spec)[0]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "actual-protocol-calls.jsonl"
            code = "\n".join([
                "import json,pathlib,sys",
                "config=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(config['family']+'\\n')",
                "assert not out.exists(); out.mkdir(parents=True)",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            result = execute_cell(cell, {"command": [sys.executable, "-c", code],
                                         "corpus": "registered-corpus.jsonl", "queries": "registered-queries.jsonl"},
                                  root / "attempt", time.monotonic_ns() + 900 * 10**9)
            self.assertEqual(calls.read_text().splitlines(), ["G-seed", "G-R"])
            self.assertFalse(result["complete"], "harmless protocol reports were accepted as positive Default qualification")
            self.assertIn("runner_only_control", result["qualification_issues"])
            self.assertEqual(json.loads((root / "attempt" / "result.json").read_text()), result)
            self.assertTrue(all(Path(step["config"]).is_file() for step in result["steps"]))


if __name__ == "__main__":
    unittest.main()

class GActualProgramIdentityTests(unittest.TestCase):
    def test_actual_program_pid_is_retained_independently_of_reported_result(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            code = "\n".join([
                "import json,os,pathlib,sys",
                "out=pathlib.Path(sys.argv[2]); assert not out.exists(); out.mkdir(parents=True)",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True,'reported_pid':os.getpid()}))",
            ])
            result = execute_cell(make_g_plan(spec)[0], {"command": [sys.executable, "-c", code],
                                                       "corpus": "registered-corpus.jsonl", "queries": "registered-queries.jsonl"},
                                  root / "attempt", time.monotonic_ns() + 900 * 10**9)
            self.assertEqual(len(result["steps"]), 2)
            for step in result["steps"]:
                self.assertEqual(step.get("pid"), step["result"]["reported_pid"],
                                 "runner did not independently retain the actual command child PID")
            self.assertNotEqual(result["steps"][0]["pid"], result["steps"][1]["pid"])


class GShortProcessObservationTests(unittest.TestCase):
    def short_ready_child(self, stage):
        from g_driver import run_program_step

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initial, ready = root / "initial", root / "ready"
            code = "\n".join([
                "import json,pathlib,sys,time",
                f"initial=pathlib.Path({str(initial)!r}); ready=pathlib.Path({str(ready)!r})",
                "deadline=time.monotonic()+3",
                "while not initial.exists() and time.monotonic()<deadline: time.sleep(0.001)",
                "assert initial.exists(); ready.touch()",
                "time.sleep(0.4)",
                "out=pathlib.Path(sys.argv[2]); out.mkdir()",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            observations = []

            def observe(pid):
                # Synthetic readiness observations qualify scheduling, never GPU engagement.
                if not observations:
                    initial.touch()
                    live_ready = False
                else:
                    stat = Path(f"/proc/{pid}/stat")
                    try:
                        state = stat.read_text().rsplit(") ", 1)[1].split()[0]
                        live_ready = ready.exists() and state not in ("Z", "X")
                    except FileNotFoundError:
                        live_ready = False
                value = {"pid": pid, "live_ready": live_ready}
                observations.append(value)
                return value

            step = run_program_step([sys.executable, "-c", code],
                                    {"family": "G-R", "stage": stage}, root / "output",
                                    time.monotonic_ns() + 5 * 10**9, observe=observe)
            self.assertTrue(step["complete"])
            self.assertEqual(step["exit"], 0)
            self.assertEqual(step["gpu_samples"], observations)
            self.assertTrue(all(item["pid"] == step["pid"] for item in observations))
            return observations

    def test_g0_observes_actual_short_child_after_readiness_before_exit(self):
        observations = self.short_ready_child("G0")
        self.assertTrue(any(item["live_ready"] for item in observations),
                        "G0 observed only pre-initialization and missed the actual live ready child")

    def test_ordinary_stage_keeps_one_second_observation_wait(self):
        observations = self.short_ready_child("G-R")
        self.assertEqual(len(observations), 1)
        self.assertFalse(observations[0]["live_ready"])
