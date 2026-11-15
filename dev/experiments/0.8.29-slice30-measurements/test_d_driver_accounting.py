"""Literal outer-wall accounting against failed prefixes; no children or stores."""

import json
from pathlib import Path
import unittest

from d_driver import run_accounted_stage


SECOND = 10**9
SPEC = json.loads(Path(__file__).with_name("d-cells.json").read_text())


class DDriverAccountingTests(unittest.TestCase):
    def test_failed_cell_charges_actual_outer_wall_once_and_refuses_remaining_dispatch(self):
        now = [1000 * SECOND]
        first = "d-c-held-n10000-r0"
        budget = {"used_ns": 14398 * SECOND, "shared_ns": 14400 * SECOND,
                  "retained_bytes": 1903142018, "storage_limit": 16 * 1024**3,
                  "cell_used_ns": {}, "inflight": None,
                  "other_phases": {"M": {"status": "HELD_COST_UNAVAILABLE", "forecast_ns": None}}}
        attempted, deadlines, records = [], [], {}

        def execute(cell, deadline_ns):
            attempted.append(cell["id"])
            deadlines.append(deadline_ns)
            now[0] += 2 * SECOND
            return {"status": "failed", "error": "literal unit failure", "elapsed_ns": SECOND,
                    "cleanup_uncertain": False, "live_owned_pids": [], "unit_diagnostic": True}

        rows = run_accounted_stage(SPEC, records, budget, execute, clock=lambda: now[0])
        self.assertEqual(attempted, [first], "consumed shared allowance permitted another preparation")
        self.assertEqual(deadlines, [1002 * SECOND], "shared remainder must bound the absolute deadline")
        self.assertEqual(budget["used_ns"], 14400 * SECOND,
                         "nested child time must not replace or duplicate whole owned elapsed")
        self.assertEqual(budget["cell_used_ns"], {first: 2 * SECOND})
        self.assertIsNone(budget["inflight"])
        self.assertEqual(budget["other_phases"]["M"],
                         {"status": "HELD_COST_UNAVAILABLE", "forecast_ns": None})
        self.assertEqual(records[first]["elapsed_ns"], 2 * SECOND)
        self.assertEqual(records[first]["child_elapsed_ns"], SECOND)
        self.assertEqual(rows[0]["status"], "failed")
        self.assertEqual([row["id"] for row in rows], [cell["id"] for cell in SPEC["cells"]])
        self.assertTrue(all(row["status"] == "not_run_cap_feasibility" for row in rows[1:4]))
        self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))


    def test_common_prefix_and_failed_cell_partition_the_single_outer_charge(self):
        now, clock_calls = [100 * SECOND], [0]
        first = "d-c-held-n10000-r0"
        budget = {"used_ns": 14395 * SECOND, "shared_ns": 14400 * SECOND,
                  "retained_bytes": 1903142018, "storage_limit": 16 * 1024**3,
                  "cell_used_ns": {}, "common_used_ns": {"G": 17 * SECOND}, "inflight": None}
        attempted = []

        def clock():
            clock_calls[0] += 1
            if clock_calls[0] == 2:
                now[0] += 3 * SECOND
            return now[0]

        def execute(cell, deadline_ns):
            attempted.append(cell["id"])
            self.assertEqual(deadline_ns, 105 * SECOND)
            now[0] += 2 * SECOND
            return {"status": "failed", "elapsed_ns": SECOND, "cleanup_uncertain": False,
                    "live_owned_pids": [], "unit_diagnostic": True}

        rows = run_accounted_stage(SPEC, {}, budget, execute, clock=clock)
        self.assertEqual(attempted, [first])
        self.assertEqual(budget["used_ns"], 14400 * SECOND)
        self.assertEqual(budget["cell_used_ns"], {first: 2 * SECOND})
        self.assertEqual(budget["common_used_ns"], {"G": 17 * SECOND, "D": 3 * SECOND})
        self.assertEqual(budget["cell_used_ns"][first] + budget["common_used_ns"]["D"],
                         5 * SECOND, "nested portions must conserve the single five-second outer wall")
        self.assertEqual(len(rows), 24)


if __name__ == "__main__":
    unittest.main()
