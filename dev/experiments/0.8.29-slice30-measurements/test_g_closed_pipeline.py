"""Actual closed prepared-job wiring; controls cannot qualify Default/GPU results."""

from contextlib import closing
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_runner
from g_reuse import collect_reuse
from test_g_closed_copy import tiny_source
from test_g_closed_forecast import observed
from test_g_closed_reuse import normal_protocol


class GClosedPipelineTests(unittest.TestCase):
    def test_calibration_uses_copy_write_route_without_seed_and_rejects_protocol_result(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _rows):
                reused = collect_reuse(current, normal)
                subject = tiny_source(root)
                calls = root / "program-calls"
                program = [sys.executable, "-c", "\n".join([
                    "import json,pathlib,sys",
                    "c=json.loads(pathlib.Path(sys.argv[1]).read_text());o=pathlib.Path(sys.argv[2]);o.mkdir()",
                    f"with pathlib.Path({str(calls)!r}).open('a') as f:f.write(c['family']+'\\n')",
                    "(o/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True)))",
                ])]
                manifest = {"preparation": "closed-copy-v1", "stage": "closed-copy-calibration",
                    "spec": current["spec"], "closed_subject": subject, "reused": reused,
                    "allocation": str(root), "commands": {"ordinary": program, "diagnostic": program},
                    "corpus": "unused-by-control", "queries": "unused-by-control"}
                with patch.object(g_runner, "observed_write_costs", return_value=observed()):
                    rows = g_runner.run_prepared(manifest, root / "prepared")
                self.assertTrue(calls.exists(), "closed calibration was wrongly refused through legacy reseed costs")
                self.assertEqual(calls.read_text().splitlines(), ["G-W"], "seed replay or later calibration ran")
                self.assertEqual(len(rows), 84)
                self.assertEqual(sum(row["status"] == "reused_complete" for row in rows), 11)
                actual = json.loads((root / "prepared/results/calibration-outcomes.json").read_text())
                self.assertEqual(actual[0]["status"], "failed", "protocol-only output falsely qualified Default")
                self.assertTrue(all(row["status"].startswith("not_run_") for row in actual[1:]))
                result = json.loads(Path(actual[0]["attempt"], "result.json").read_text())
                self.assertIn("runner_only_control", result["qualification_issues"])
                self.assertIn("actual_reserved_gpu_engagement_unobserved", result["qualification_issues"])
                self.assertTrue(result["preparation"]["validated"])
                with closing(sqlite3.connect(subject["files"][""]["path"])) as connection:
                    self.assertEqual(connection.execute("SELECT body FROM docs").fetchone()[0], "original")
