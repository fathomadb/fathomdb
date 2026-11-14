"""Actual child order at the proposed normal-controller stage boundary."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from g_campaign import make_g_plan, run_g_plan


class GClosedStageTests(unittest.TestCase):
    def test_branch_stage_stops_after_named_child_with_all_obligations_retained(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = root / "original"
            original.mkdir()
            references = {}
            for cell in make_g_plan(spec):
                if cell["id"] not in {"g0-smoke", "g1-n10000", "g-r-n10000-r0", "g-r-n10000-r1"}:
                    continue
                attempt = original / cell["id"]
                attempt.mkdir()
                references[cell["id"]] = {"id": cell["id"], "cell": cell,
                    "status": "reused_complete", "elapsed_ns": 10**6, "attempt": str(attempt),
                    "origin": {"root": str(original), "source_sha": "control-source", "campaign_uuid": "control-uuid"}}
            calls = root / "calls.jsonl"
            command = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sys",
                "cell=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(cell['id']+'\\n')",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])]
            outcomes = run_g_plan(spec, root / "new-stage", command, allocation_root=root,
                                  reused=references, stage="branch-controls")
            self.assertEqual(calls.read_text().splitlines(), ["g-o-branches"],
                             "normal controller dispatched outside the closed branch stage")
            self.assertEqual(len(outcomes), 84)
            actual = {row["id"]: row for row in outcomes}
            self.assertEqual(actual["g-o-branches"]["status"], "complete")
            for identifier, row in actual.items():
                if identifier in references:
                    self.assertEqual(row, references[identifier])
                elif identifier != "g-o-branches":
                    self.assertEqual(row["status"], "not_run_stage")
                    self.assertFalse(row["complete"])
            budget = json.loads((root / "g-budget.json").read_text())
            self.assertEqual(set(budget["cell_used_ns"]), {"g-o-branches"})
            self.assertIsNone(budget["inflight"])
