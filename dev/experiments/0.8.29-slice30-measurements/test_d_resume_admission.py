"""Attempted D outcomes survive prospective refusals; unstarted rows stay eligible."""

import copy
import json
from pathlib import Path
import unittest

from d_driver import run_accounted_stage


SECOND = 10**9
SPEC = json.loads(Path(__file__).with_name("d-cells.json").read_text())


class DResumeAdmissionTests(unittest.TestCase):
    def test_recorded_failure_survives_cap_stage_review_and_missing_lower_evidence(self):
        cases = [
            ("d-c-held-n10000-r0", True, 14400, "d-t-held-n10000-r0", "not_run_cap_feasibility"),
            ("d-c-overlap-n10000-r0", True, 42, "d-t-overlap-n10000-r0", "not_run_stage_review"),
            ("d-c-held-n100000-r0", False, 42, "d-t-held-n100000-r0", "not_run_lower_evidence"),
        ]
        for cell_id, initial, used_seconds, peer_id, peer_status in cases:
            with self.subTest(cell=cell_id, refusal=peer_status):
                previous = {"status": "failed", "error": "original owned attempt failed",
                            "elapsed_ns": 2 * SECOND, "child_elapsed_ns": None,
                            "cleanup_uncertain": True, "live_owned_pids": None,
                            "unit_diagnostic": True}
                records = {cell_id: previous}
                budget = {"used_ns": used_seconds * SECOND, "shared_ns": 14400 * SECOND,
                          "retained_bytes": 1903142018, "storage_limit": 16 * 1024**3,
                          "cell_used_ns": {cell_id: 2 * SECOND}, "common_used_ns": {"D": SECOND},
                          "inflight": {"cell": cell_id, "status": "cleanup_uncertain"},
                          "other_phases": {"M": {"forecast_ns": None}}}
                before_records, before_budget = copy.deepcopy(records), copy.deepcopy(budget)
                attempted = []

                def execute(cell, _deadline):
                    attempted.append(cell["id"])
                    raise AssertionError("no unresolved-owned execution may be retried or replaced")

                rows = run_accounted_stage(SPEC, records, budget, execute,
                                           initial=initial, clock=lambda: 100 * SECOND)
                actual = {row["id"]: row for row in rows}
                self.assertEqual(actual[cell_id], {"id": cell_id, **previous},
                                 "prospective refusal replaced the actual failed attempt")
                self.assertEqual(actual[peer_id]["status"], peer_status)
                self.assertEqual(attempted, [])
                self.assertEqual(records, before_records)
                self.assertEqual(budget, before_budget, "resume reset/recharged prior wall or ownership")
                self.assertEqual([row["id"] for row in rows], [cell["id"] for cell in SPEC["cells"]])

    def test_unstarted_not_run_rows_are_reassessed_instead_of_frozen_as_attempts(self):
        initial_ids = [cell["id"] for cell in SPEC["cells"][:4]]
        records = {cell_id: {"status": "not_run_stage_review", "reason": "prior unstarted hold",
                             "elapsed_ns": None} for cell_id in initial_ids}
        budget = {"used_ns": 40 * SECOND, "shared_ns": 14400 * SECOND,
                  "retained_bytes": 1903142018, "storage_limit": 16 * 1024**3,
                  "cell_used_ns": {}, "inflight": None}
        now, attempted = [100 * SECOND], []

        def execute(cell, _deadline):
            attempted.append(cell["id"])
            now[0] += SECOND
            return {"status": "failed", "error": "first actual unit attempt", "elapsed_ns": SECOND,
                    "cleanup_uncertain": False, "live_owned_pids": [], "unit_diagnostic": True}

        rows = run_accounted_stage(SPEC, records, budget, execute, clock=lambda: now[0])
        self.assertEqual(attempted, initial_ids, "unstarted holds were treated as executed attempts")
        self.assertTrue(all(row["status"] == "failed" for row in rows[:4]))
        self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))
        self.assertEqual(budget["used_ns"], 44 * SECOND)
        self.assertEqual(budget["cell_used_ns"], {cell_id: SECOND for cell_id in initial_ids})
        self.assertEqual(len(rows), 24)


if __name__ == "__main__":
    unittest.main()
