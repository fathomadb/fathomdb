"""Whole-cell deadline includes native report decoding/qualification wall."""

import unittest
from unittest.mock import patch

from d_driver import run_accounted_stage
from d_report import bind_execution
from test_d_execution_routing import SECOND, SPEC, native_report


class DReportDeadlineTests(unittest.TestCase):
    def test_report_cost_overrunning_remaining_allowance_is_charged_but_never_qualified(self):
        first, ordinary = SPEC["cells"][0]["id"], SPEC["cells"][1]["id"]
        records = {first: {"status": "failed", "error": "prior C failure", "elapsed_ns": 2*SECOND}}
        budget = {"used_ns": 14398*SECOND, "shared_ns": 14400*SECOND,
                  "retained_bytes": 1903142018, "storage_limit": 16*1024**3,
                  "cell_used_ns": {first: 2*SECOND}, "inflight": None}
        now, called = [100*SECOND], []

        def execute(cell, deadline):
            called.append(cell["id"])
            self.assertEqual(deadline, 102*SECOND)
            now[0] += SECOND
            return {"status": "complete", "attempt_id": "one", "native": native_report(cell, "one"),
                    "peak_bytes": 256*1024**2, "elapsed_ns": SECOND,
                    "cleanup_uncertain": False, "live_owned_pids": [], "unit_diagnostic": True}

        def decoding_and_binding(*args, **kwargs):
            value = bind_execution(*args, **kwargs)
            now[0] += 2*SECOND
            return value

        with patch("d_driver.bind_execution", side_effect=decoding_and_binding):
            rows = run_accounted_stage(SPEC, records, budget, execute, clock=lambda: now[0])
        self.assertEqual(called, [ordinary], "post-report overrun allowed another dispatch")
        self.assertEqual(rows[1]["status"], "incomplete", "late report was published as qualified lower evidence")
        self.assertTrue(rows[1]["deadline_exceeded"])
        self.assertTrue(rows[1]["execution_report"]["complete"], "native closure remains observed despite cap failure")
        self.assertEqual(rows[1]["elapsed_ns"], 3*SECOND)
        self.assertEqual(rows[1]["child_elapsed_ns"], SECOND)
        self.assertEqual(budget["used_ns"], 14401*SECOND, "actual overrun was clamped or double charged")
        self.assertEqual(budget["cell_used_ns"], {first: 2*SECOND, ordinary: 3*SECOND})
        self.assertIsNone(budget["inflight"])
        self.assertTrue(all(row["status"] == "not_run_cap_feasibility" for row in rows[2:4]))
        self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))
        self.assertEqual(len(rows), 24)


if __name__ == "__main__":
    unittest.main()
