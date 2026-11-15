"""Failed scientific obligations remain visible without automatic re-execution."""

import json
from pathlib import Path
import unittest

from d_driver import run_accounted_stage


SECOND = 10**9
SPEC = json.loads(Path(__file__).with_name("d-cells.json").read_text())


class DResumeTests(unittest.TestCase):
    def test_failed_attempt_resumes_without_retry_and_preserves_original_error_and_charge(self):
        first = "d-c-held-n10000-r0"
        previous = {"status": "failed", "error": "prior literal failure", "elapsed_ns": 2 * SECOND,
                    "child_elapsed_ns": SECOND, "cleanup_uncertain": False,
                    "live_owned_pids": [], "unit_diagnostic": True}
        records = {first: dict(previous)}
        budget = {"used_ns": 42 * SECOND, "shared_ns": 14400 * SECOND,
                  "retained_bytes": 1903142018, "storage_limit": 16 * 1024**3,
                  "cell_used_ns": {first: 2 * SECOND}, "inflight": None}
        now, attempted = [100 * SECOND], []

        def execute(cell, _deadline):
            attempted.append(cell["id"])
            now[0] += SECOND
            return {"status": "failed", "error": "new literal failure", "elapsed_ns": SECOND,
                    "cleanup_uncertain": False, "live_owned_pids": [], "unit_diagnostic": True}

        rows = run_accounted_stage(SPEC, records, budget, execute, clock=lambda: now[0])
        self.assertEqual(attempted, ["d-t-held-n10000-r0", "d-t-held-n10000-r1",
                                     "d-c-held-n10000-r1"], "resume retried a recorded failed cell")
        self.assertEqual(records[first], previous, "resume overwrote the actual failed prefix")
        self.assertEqual(rows[0], {"id": first, **previous})
        self.assertEqual(budget["used_ns"], 45 * SECOND)
        self.assertEqual(budget["cell_used_ns"][first], 2 * SECOND)
        self.assertEqual(len(rows), 24)
        self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))
        attempted.clear()
        again = run_accounted_stage(SPEC, records, budget, execute, clock=lambda: now[0])
        self.assertEqual(attempted, [], "all failed initial attempts were automatically retried")
        self.assertEqual(again, rows)
        self.assertEqual(budget["used_ns"], 45 * SECOND, "resume recharged retained attempt walls")


if __name__ == "__main__":
    unittest.main()
