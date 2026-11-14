"""A physical copy alone cannot admit R1 without actual qualified calibration costs."""

from pathlib import Path
import sys
import tempfile
import unittest

import g_runner
from g_reuse import collect_reuse
from test_g_closed_copy import tiny_source
from test_g_closed_reuse import normal_protocol


class GClosedR1AdmissionTests(unittest.TestCase):
    def test_missing_matched_calibration_prevents_real_program_dispatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _rows):
                marker = root / "forbidden"
                command = [sys.executable, "-c", "\n".join([
                    "import json,pathlib,sys",
                    f"pathlib.Path({str(marker)!r}).touch()",
                    "o=pathlib.Path(sys.argv[2]);o.mkdir();(o/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True)))",
                ])]
                manifest = {"preparation": "closed-copy-v1", "stage": "closed-copy-read-r1",
                    "spec": current["spec"], "closed_subject": tiny_source(root),
                    "reused": collect_reuse(current, normal), "allocation": str(root),
                    "commands": {"ordinary": command, "diagnostic": command},
                    "corpus": "unused-control", "queries": "unused-control"}
                rows = g_runner.run_prepared(manifest, root / "prepared")
                self.assertFalse(marker.exists(), "R1 dispatched from copy availability without matched calibration forecast")
                r1 = next(row for row in rows if row["id"] == "g-r-n100000-r1")
                self.assertEqual(r1["status"], "not_run_prerequisite")
                self.assertEqual(r1["reason"], "qualified_calibration_costs_required")
                self.assertEqual(len(rows), 84)
                self.assertTrue(all(row["status"] == "not_run_stage" for row in rows if row["cell"]["stage"] == "G-W"))
