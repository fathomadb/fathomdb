"""Manual native-report routing through fixed24 accounting; no native execution."""

import json
from pathlib import Path
import unittest

from d_driver import run_accounted_stage


SECOND = 10**9
SPEC = json.loads(Path(__file__).with_name("d-cells.json").read_text())


def native_report(cell, attempt):
    """State the fixed10k ordinary contract; these literals are unit data only."""
    identity = {"cell_id": cell["id"], "attempt_id": attempt, "n": cell["n"],
                "mode": cell["mode"], "layer": cell["layer"], "repeat": cell["repeat"]}
    return {"cell": identity, "complete": True, "close_succeeded": True,
            "collection_enabled": False, "periodic_numeric_polls": 0, "periodic_readiness_polls": 0,
            "runtime": {"scheduler_workers": 2, "embed_workers": 5, "admission_capacity": 128},
            "batches": [{"rows": 1000, "cursor": cursor} for cursor in range(1000, 10001, 1000)],
            "final": {"generation_id": "unit-generation", "pending_count": 0,
                      "generation_pending_count": 0, "generation_failed_count": 0,
                      "natural_rows": 10000, "physical_rows": 10000, "vector_rows": 10000,
                      "span_rows": 10000, "terminal_rows": 10000, "failed_rows": 0},
            "calls": [{"sample_index": index, "query_index": index % 500, "route": "engine_hybrid",
                       "limit": 10, "ok": True, "started_ns": index*3, "ended_ns": index*3+1}
                      for index in range(1000)]}


def run_manual_result(change=None):
    """Route four initial literal results and retain all20 subsequent holds."""
    now, records, calls = [100*SECOND], {}, []
    budget = {"used_ns": 40*SECOND, "shared_ns": 14400*SECOND, "retained_bytes": 1903142018,
              "storage_limit": 16*1024**3, "cell_used_ns": {}, "inflight": None}

    def execute(cell, _deadline):
        calls.append(cell["id"])
        now[0] += SECOND
        value = {"status": "complete", "attempt_id": "unit-one", "native": native_report(cell, "unit-one"),
                 "peak_bytes": 256*1024**2, "elapsed_ns": SECOND//2, "cleanup_uncertain": False,
                 "live_owned_pids": [], "unit_diagnostic": True}
        if change:
            change(value)
        return value

    rows = run_accounted_stage(SPEC, records, budget, execute, clock=lambda: now[0])
    return rows, records, budget, calls


class DExecutionRoutingTests(unittest.TestCase):
    def test_t_result_is_bound_before_becoming_lower_evidence_and_missing_c_stays_incomplete(self):
        rows, records, budget, calls = run_manual_result()
        ordinary = rows[1]
        self.assertEqual(ordinary.get("physical_count"), 10000, "native T closure never reached the controller record")
        self.assertTrue(ordinary.get("closed"))
        self.assertEqual(ordinary["error_count"], 0)
        self.assertEqual(ordinary["status"], "complete")
        self.assertTrue(ordinary["execution_report"]["complete"])
        self.assertEqual(ordinary["execution_report"]["overlap"]["verdict"], "ACTUAL_OVERLAP_UNOBSERVED")
        self.assertIsNone(ordinary["actual_overlap"])
        self.assertIsNone(ordinary["observations_complete"])
        self.assertEqual(ordinary["elapsed_ns"], SECOND)
        self.assertEqual(ordinary["child_elapsed_ns"], SECOND//2)
        self.assertEqual(ordinary["peak_bytes"], 256*1024**2)
        self.assertEqual(rows[0]["status"], "incomplete", "missing C capture was accepted as complete")
        self.assertFalse(rows[0]["observations_complete"])
        self.assertEqual(budget["used_ns"], 44*SECOND)
        self.assertEqual(calls, [cell["id"] for cell in SPEC["cells"][:4]])
        self.assertEqual(records[ordinary["id"]], {key: value for key, value in ordinary.items() if key != "id"})
        self.assertEqual(len(rows), 24)
        self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))

    def test_success_without_native_report_does_not_become_qualified_lower_evidence(self):
        rows, _, _, _ = run_manual_result(lambda value: value.pop("native"))
        self.assertTrue(all(row["status"] == "incomplete" for row in rows[:4]),
                        "claimed child success bypassed current native/report qualification")

    def test_physical_gap_cannot_qualify_t_and_unknown_cost_does_not_become_zero(self):
        def physical_gap(value):
            value["native"]["final"]["span_rows"] = 9999

        rows, _, _, _ = run_manual_result(physical_gap)
        self.assertEqual(rows[1]["status"], "incomplete", "physical gap was published as lower evidence")
        self.assertFalse(rows[1]["execution_report"]["complete"])
        rows, _, _, _ = run_manual_result(lambda value: value.update(peak_bytes=None))
        self.assertEqual(rows[1]["status"], "incomplete", "unknown retained cost qualified a completed cell")
        self.assertIsNone(rows[1]["peak_bytes"], "unknown observed storage became zero")


if __name__ == "__main__":
    unittest.main()
