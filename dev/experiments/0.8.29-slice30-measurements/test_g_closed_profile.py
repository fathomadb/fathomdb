"""Prospective reviewed totals fund only new writes; original12 subjects stay exact."""

import json
from pathlib import Path
import tempfile
import unittest

from g_campaign import make_g_plan, run_g_plan
from test_g_closed_modes import harmless
from test_g_closed_write_gate import setup


class GClosedProfileTests(unittest.TestCase):
    def test_reviewed_remaining_allowance_funds_new_writes_without_changing_references(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = setup()
            spec["_closed_limits"] = {"campaign_seconds": 43200, "active_bytes": 128*1024**3}
            spec["_closed_observation_wait_ms"] = 50
            review = spec["_reviewed_costs"]
            review["limits"] = spec["_closed_limits"]
            plan = make_g_plan(spec)
            review["g_table"] = {cell["id"]: {"cell": cell, "prediction": {
                "allowed": True, "total_ns": 1, "new_bytes": 1}}
                for cell in plan if cell["stage"] == "G-W"}
            token = {"consumed": True, "source": "original-one-shot"}
            (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 15000*10**9, "inflight": None, "exploratory_seed100k": token}))
            rows = run_g_plan(spec, root / "results", harmless(root / "calls"),
                              allocation_root=root, reused=reused)
            self.assertTrue((root / "calls").exists(), "reviewed remaining allowance still uses exhausted historical4h cap")
            self.assertEqual(len((root / "calls").read_text().splitlines()), 72)
            self.assertEqual(len(rows), 84)
            writes = [row for row in rows if row["cell"]["stage"] == "G-W"]
            self.assertTrue(all(row["cell"]["cell_seconds"] == 1500 for row in writes))
            self.assertTrue(all(row["cell"]["campaign_seconds"] == 43200 for row in writes))
            self.assertTrue(all(row["cell"]["active_bytes"] == 128*1024**3 for row in writes))
            self.assertTrue(all(row["cell"]["observation_wait_ms"] == 50 for row in writes))
            for identifier, reference in reused.items():
                self.assertEqual(next(row for row in rows if row["id"] == identifier), reference)
            budget = json.loads((root / "g-budget.json").read_text())
            self.assertGreater(budget["used_ns"], 15000*10**9)
            self.assertEqual(budget["exploratory_seed100k"], token)
            self.assertIsNone(budget["inflight"])
