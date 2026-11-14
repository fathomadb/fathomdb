"""Closed controller boundaries with actual harmless children, never G measurements."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from g_campaign import make_g_plan, run_g_plan
from test_g_closed_plan import ORIGINAL, SPEC


def references():
    return {cell["id"]: {"id": cell["id"], "cell": cell, "status": "reused_complete",
                         "origin": {"root": "protocol-original"}, "elapsed_ns": 1}
            for cell in make_g_plan(SPEC) if cell["id"] in ORIGINAL}


def harmless(calls):
    return [sys.executable, "-c", "\n".join([
        "import json,pathlib,sys",
        "cell=json.loads(pathlib.Path(sys.argv[1]).read_text());out=pathlib.Path(sys.argv[2])",
        f"with pathlib.Path({str(calls)!r}).open('a') as stream:stream.write(cell['id']+'\\n')",
        "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
    ])]


class GClosedModeTests(unittest.TestCase):
    def test_r1_mode_stops_before_any_write_even_after_success(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "calls"
            spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-read-r1")
            rows = run_g_plan(spec, root / "results", harmless(calls), allocation_root=root,
                              reused=references())
            self.assertEqual(calls.read_text().splitlines(), ["g-r-n100000-r1"],
                             "R1 success automatically dispatched write work before cost review")
            self.assertEqual(len(rows), 84)
            self.assertEqual(sum(row["status"] == "reused_complete" for row in rows), 11)
            self.assertEqual(sum(row["status"] == "complete" for row in rows), 1)
            self.assertTrue(all(row["status"] == "not_run_stage" for row in rows
                                if row["cell"]["stage"] == "G-W"))
            again = run_g_plan(spec, root / "results", harmless(calls), allocation_root=root,
                               reused=references())
            self.assertEqual(calls.read_text().splitlines(), ["g-r-n100000-r1"])
            self.assertEqual(len(again), 84)
