"""Real harmless-process fixed24 connection and durable custody; no D database run."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from d_driver import run_supervised_stage


SPEC = json.loads(Path(__file__).with_name("d-cells.json").read_text())


class DSupervisedStageTests(unittest.TestCase):
    def test_initial_four_are_owned_and_durable_then_resume_retains_all24_without_retry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ledger = root/"g-budget.json"
            markers = root/"actual-dispatches"
            initial_used = 1845130877940
            budget = {"schema": "slice30-gd-budget/v1", "used_ns": initial_used,
                      "shared_ns": 14400*10**9, "retained_bytes": 1903142018,
                      "storage_limit": 16*1024**3, "cell_used_ns": {}, "inflight": None,
                      "other_phases": {"M": {"status": "HELD_COST_UNAVAILABLE", "forecast_ns": None}}}
            records = {}

            def publish():
                ledger.write_text(json.dumps(budget))

            publish()
            code = "\n".join([
                "import json,pathlib,sys",
                "config=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                f"ledger=json.loads(pathlib.Path({str(ledger)!r}).read_text())",
                "assert ledger['inflight']['cell']==config['id'], 'D custody was not durable before child start'",
                f"with pathlib.Path({str(markers)!r}).open('a') as stream: stream.write(config['id']+'\\n')",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':False,'unit_diagnostic':True}))",
            ])
            command = [sys.executable, "-c", code]
            rows = run_supervised_stage(SPEC, records, budget, root/"attempts", command, publish=publish)
            self.assertTrue(markers.exists(), "fixed24 driver never connected actual owned dispatch")
            self.assertEqual(markers.read_text().splitlines(), [
                "d-c-held-n10000-r0", "d-t-held-n10000-r0",
                "d-t-held-n10000-r1", "d-c-held-n10000-r1"])
            self.assertEqual([row["id"] for row in rows], [cell["id"] for cell in SPEC["cells"]])
            self.assertTrue(all(row["status"] == "failed" for row in rows[:4]))
            self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))
            self.assertIsNone(budget["inflight"])
            self.assertEqual(json.loads(ledger.read_text()), budget)
            self.assertGreater(budget["used_ns"], initial_used)
            self.assertEqual(budget["used_ns"]-initial_used,
                sum(budget["cell_used_ns"].values())+budget.get("common_used_ns", {}).get("D", 0))
            self.assertEqual(budget["other_phases"]["M"],
                {"status": "HELD_COST_UNAVAILABLE", "forecast_ns": None})
            before_cells, before_records = dict(budget["cell_used_ns"]), dict(records)
            resumed = run_supervised_stage(SPEC, records, budget, root/"attempts", command, publish=publish)
            self.assertEqual(markers.read_text().splitlines(), [row["id"] for row in rows[:4]])
            self.assertEqual(budget["cell_used_ns"], before_cells)
            self.assertEqual(records, before_records)
            self.assertEqual([row["status"] for row in resumed], [row["status"] for row in rows])
            self.assertEqual(len(resumed), 24)
            self.assertEqual(json.loads(ledger.read_text()), budget)


if __name__ == "__main__":
    unittest.main()
