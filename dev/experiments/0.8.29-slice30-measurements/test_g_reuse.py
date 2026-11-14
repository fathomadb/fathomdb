"""Resume scheduling from literal prior records; these are not GPU qualification."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from g_campaign import make_g_plan, run_g_plan


class GOriginalResultReuseTests(unittest.TestCase):
    def test_four_original_records_prevent_actual_redispatch_and_preserve_charge(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        selected = {"g0-smoke", "g1-n10000", "g-r-n10000-r0", "g-r-n10000-r1"}
        with tempfile.TemporaryDirectory() as temporary:
            allocation = Path(temporary)
            original, successor = allocation / "original", allocation / "successor"
            original.mkdir()
            calls = allocation / "calls.jsonl"
            command = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sys",
                "cell=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(cell['id']+'\\n')",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])]
            references = {}
            for cell in make_g_plan(spec):
                if cell["id"] not in selected:
                    continue
                attempt = original / cell["id"] / "attempt-0001"
                attempt.mkdir(parents=True)
                (attempt / "result.json").write_text('{"complete":true,"runner_only":true}\n')
                references[cell["id"]] = {
                    "id": cell["id"], "cell": cell, "status": "reused_complete",
                    "attempt": str(attempt), "elapsed_ns": 10**6,
                    "origin": {"root": str(original), "source_sha": "original-source",
                               "campaign_uuid": "original-uuid"},
                }
            retained = {str(path): path.read_bytes() for path in original.rglob("*") if path.is_file()}
            charged = {identifier: 123456789 for identifier in selected}
            (allocation / "g-budget.json").write_text(json.dumps({
                "schema": "slice30-gd-budget/v1", "used_ns": 357099039661,
                "cell_used_ns": charged, "inflight": None}))
            outcomes = run_g_plan(spec, successor, command, allocation_root=allocation, reused=references)
            actual = calls.read_text().splitlines()
            self.assertTrue(selected.isdisjoint(actual),
                            "accepted original cases were actually dispatched again")
            self.assertEqual(len(outcomes), 84)
            self.assertEqual(len(actual), 80)
            rows = {row["id"]: row for row in outcomes}
            for identifier in selected:
                self.assertEqual(rows[identifier], references[identifier])
            self.assertEqual({str(path): path.read_bytes() for path in original.rglob("*") if path.is_file()}, retained)
            budget = json.loads((allocation / "g-budget.json").read_text())
            self.assertGreater(budget["used_ns"], 357099039661)
            self.assertIsNone(budget["inflight"])
            for identifier, prior in charged.items():
                self.assertEqual(budget["cell_used_ns"][identifier], prior)
            state = {row["id"]: row for row in json.loads((successor / "campaign-state.json").read_text())}
            for identifier in selected:
                self.assertEqual(state[identifier], references[identifier])
