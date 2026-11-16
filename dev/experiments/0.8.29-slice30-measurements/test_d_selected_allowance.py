"""Literal per-size D allowance admission; no new scientific execution."""

import copy
import unittest

from d_controller import assess_stage
from d_driver import reviewed_quartet
from test_d_continuation import fixture, SPEC, SECOND, MIB


class DSelectedAllowanceTests(unittest.TestCase):
    def next100k(self):
        records, budget, review = fixture(overlap_complete=True)
        for identity, row in records.items():
            row["elapsed_ns"] = (72 if identity.startswith("d-c-") else 71) * SECOND
            row["peak_bytes"] = 32 * MIB
        review["bounds"] = {"C": {"time_ns": 1440 * SECOND, "active_bytes": 640 * MIB},
                            "T": {"time_ns": 1420 * SECOND, "active_bytes": 640 * MIB}}
        return records, budget, review

    def test_measured_lower_bound_fits_selected1500s100k_quartet(self):
        records, budget, review = self.next100k()
        before = copy.deepcopy(records)
        assessment = assess_stage(100000, "held", records, budget)
        self.assertEqual(assessment["status"], "admit", "100k still inherits obsolete900s cap")
        decision = reviewed_quartet(SPEC, records, budget, review)
        self.assertEqual(decision["status"], "admit", "reviewed100k1500s path still refuses")
        self.assertEqual(decision["quartet_time_ns"], 5720 * SECOND)
        self.assertEqual(len(decision["cells"]), 4)
        self.assertEqual(records, before, "allowance amendment rewrote old qualified evidence")
        self.assertEqual(assess_stage(300000, "held", records, budget)["status"], "not_run_lower_evidence")

    def test_fixed_inventory_keeps10k900_and_selects100k1500_without_changing24cells(self):
        self.assertEqual({cell["cell_ns"] for cell in SPEC["cells"] if cell["n"] == 100000},
                         {1500 * SECOND}, "100k execution deadlines do not use selected allowance")
        self.assertEqual({cell["cell_ns"] for cell in SPEC["cells"] if cell["n"] == 10000}, {900 * SECOND})
        self.assertEqual(len(SPEC["cells"]), 24)
        records, budget, review = self.next100k()
        review["bounds"]["C"]["time_ns"] = 1500 * SECOND
        self.assertNotEqual(reviewed_quartet(SPEC, records, budget, review)["status"], "admit")


if __name__ == "__main__":
    unittest.main()
