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
