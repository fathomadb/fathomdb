"""Hand two-group D attribution and overlap; no native/campaign evidence."""

import copy
import unittest

from d_report import bind_execution


PHASES = ("projection_commit_gate_wait", "projection_commit_begin", "projection_commit_prepare",
          "projection_commit_publication", "projection_commit_sqlite", "projection_commit_completion")


def literal_execution():
    """Use two cursors/groups and the sole witness query[10,30]/worker[2,40]/pending[15,20]."""
    cell = {"cell_id": "tiny-c-overlap-n2-r0", "attempt_id": "one", "n": 2,
            "repeat": 0, "layer": "C", "mode": "overlap"}
    actual = {"cell": cell, "complete": True, "close_succeeded": True,
              "collection_enabled": True, "periodic_numeric_polls": 1, "periodic_readiness_polls": 1,
              "runtime": {"scheduler_workers": 2, "embed_workers": 5, "admission_capacity": 128},
              "batches": [{"rows": 2, "cursor": 102}],
              "final": {"generation_id": "generation-one", "pending_count": 0,
                        "generation_pending_count": 0, "generation_failed_count": 0,
                        "natural_rows": 2, "physical_rows": 2, "vector_rows": 2, "span_rows": 2,
                        "terminal_rows": 2, "failed_rows": 0},
              "calls": [{"sample_index": i, "query_index": i % 500, "operation_id": 17+i,
                         "route": "engine_hybrid", "limit": 10, "ok": True,
                         "started_ns": 9+40*i, "ended_ns": 31+40*i} for i in range(1000)]}
    frames = []

    def append(phase, operation, span, start, end, facts=None, parent=None):
        frames.append({"cell": copy.deepcopy(cell), "record": {
            "engine_id": 42, "session_id": 7, "operation_id": operation,
            "span_id": span, "parent_span_id": parent, "phase": phase, "status": "ok",
            "started_ns": start, "ended_ns": end, "facts": facts or {},
        }})

    scan = 1 << 63
    append("projection_scan", scan, 1, 0, 1, {"cursors": [101, 102], "runtime_index": 0})
    for index, cursor in enumerate((101, 102)):
        operation, base, start = scan+1+index, 10+20*index, 2+40*index
        append("projection_dequeue", operation, base, None, None,
               {"cursors": [cursor], "worker_index": index, "scan_operation_id": scan})
        append("projection_embed_dispatch_wall", operation, base+1, start+1, start+2,
               {"cursors": [cursor], "runtime_index": None}, parent=base+9)
        for offset, phase in enumerate(PHASES):
            append(phase, operation, base+2+offset, start+2+offset, start+3+offset,
                   {"cursors": [cursor], "runtime_index": index}, parent=base+9)
        append("projection_commit_outcome", operation, base+8, None, None,
               {"cursors": [cursor], "generation_id": "generation-one", "input_success": 1,
                "input_failure": 0, "input_deferred": 0, "generation_skips": 0,
                "member_skips": 0, "terminal_skips": 0, "staged_success": 1,
                "staged_failure": 0, "commit_attempted": True, "committed_success": 1,
                "committed_failure": 0, "publication_state": "committed"}, parent=base+9)
        append("projection_processing", operation, base+9, start, 40+42*index,
               {"cursors": [cursor], "runtime_index": index})
    for i in range(1000):
        append("api_hybrid", 17+i, 1000+i, 10+40*i, 30+40*i)
    observation = {"engine_id": 42, "session_id": 7, "dropped_records": 0, "issues": [],
                   "records": frames, "pending": [{"cell": copy.deepcopy(cell), "engine_id": 42,
                       "session_id": 7, "started_ns": 15, "ended_ns": 20, "pending_count": 1}]}
    return cell, actual, observation


class DReportTests(unittest.TestCase):
    def test_two_current_groups_require_durable_cursor_closure_and_exact_overlap(self):
        cell, actual, observation = literal_execution()
        report = bind_execution(cell, actual, observation)
        self.assertTrue(report["complete"], "current execution's complete observation was not bound")
        self.assertTrue(report["observations_complete"])
        self.assertEqual(report["committed_success_rows"], 2)
        self.assertEqual(report["committed_cursors"], [101, 102])
        self.assertEqual(report["worker_groups"], 2)
        self.assertTrue(report["actual_overlap"])
        self.assertEqual(report["overlap"]["operation_id"], 17)
        self.assertEqual(report["overlap"]["pending_bounds_ns"], [15, 20])
        self.assertEqual(report["phases"]["projection_processing"]["inclusive_mean_ns"], 39)
        self.assertEqual(report["phases"]["projection_processing"]["exclusive_remainder_mean_ns"], 32)
        self.assertEqual(report["phases"]["projection_commit_sqlite"]["inclusive_mean_ns"], 1)
        self.assertNotIn("projection_dequeue", report["phases"], "untimed dequeue became a zero-cost span")
        self.assertNotIn("projection_commit_outcome", report["phases"])

    def test_missing_group_phase_loss_unknown_commit_and_wrong_generation_are_incomplete(self):
        for defect in ("missing_group_phase", "loss", "unknown_commit", "wrong_generation", "physical_gap"):
            with self.subTest(defect=defect):
                cell, actual, observation = literal_execution()
                if defect == "missing_group_phase":
                    observation["records"] = [frame for frame in observation["records"]
                        if not (frame["record"]["phase"] == "projection_commit_sqlite"
                                and frame["record"]["operation_id"] == (1 << 63)+2)]
                elif defect == "loss":
                    observation["dropped_records"] = 1
                elif defect == "physical_gap":
                    actual["final"]["span_rows"] = 1
                else:
                    outcome = next(frame["record"] for frame in observation["records"]
                        if frame["record"]["phase"] == "projection_commit_outcome"
                        and frame["record"]["operation_id"] == (1 << 63)+2)
                    if defect == "unknown_commit":
                        outcome["facts"]["committed_success"] = None
                        outcome["facts"]["publication_state"] = "unavailable"
                    else:
                        outcome["facts"]["generation_id"] = "previous-generation"
                report = bind_execution(cell, actual, observation)
                self.assertFalse(report["complete"], "partial/stale observation qualified a complete cell")
                if defect == "unknown_commit":
                    self.assertIsNone(report["committed_success_rows"], "unavailable durable work became zero")

    def test_later_natural_operation_can_supply_only_its_own_witness(self):
        cell, actual, observation = literal_execution()
        observation["pending"][0].update(started_ns=55, ended_ns=60)
        report = bind_execution(cell, actual, observation)
        self.assertTrue(report["complete"])
        self.assertEqual(report["overlap"]["operation_id"], 18)
        self.assertEqual(report["overlap"]["query_bounds_ns"], [50, 70])
        self.assertEqual(report["overlap"]["worker_bounds_ns"], [42, 82])

    def test_held_characterization_requires_actual_phase_and_pending_observations(self):
        for defect in ("not_run_commit", "missing_pending"):
            with self.subTest(defect=defect):
                cell, actual, observation = literal_execution()
                cell.update(cell_id="tiny-c-held-n2-r0", mode="held")
                for frame in observation["records"]:
                    frame["cell"] = copy.deepcopy(cell)
                for sample in observation["pending"]:
                    sample["cell"] = copy.deepcopy(cell)
                if defect == "not_run_commit":
                    phase = next(frame["record"] for frame in observation["records"]
                        if frame["record"]["phase"] == "projection_commit_sqlite"
                        and frame["record"]["operation_id"] == (1 << 63)+2)
                    phase.update(status="not_run", started_ns=None, ended_ns=None)
                else:
                    observation["pending"] = []
                report = bind_execution(cell, actual, observation)
                self.assertFalse(report["complete"], "missing required C-H observation qualified")
                self.assertFalse(report["observations_complete"])
                self.assertEqual(report["phases"]["projection_commit_sqlite"]["inclusive_mean_ns"], 1)

    def test_no_qualifying_pending_keeps_overlap_obligation_unsatisfied(self):
        cell, actual, observation = literal_execution()
        observation["pending"][0]["pending_count"] = 0
        report = bind_execution(cell, actual, observation)
        self.assertFalse(report["complete"])
        self.assertEqual(report["overlap"]["verdict"], "NO_OBSERVED_OVERLAP")

    def test_ordinary_layer_never_borrows_characterization_attribution(self):
        cell, actual, observation = literal_execution()
        ordinary = {**cell, "cell_id": "tiny-t-overlap-n2-r0", "layer": "T", "attempt_id": "two"}
        actual.update(cell=ordinary, collection_enabled=False, periodic_numeric_polls=0,
                      periodic_readiness_polls=0)
        report = bind_execution(ordinary, actual, observation)
        self.assertTrue(report["complete"])
        self.assertIsNone(report["observations_complete"])
        self.assertIsNone(report["committed_success_rows"])
        self.assertIsNone(report["actual_overlap"])
        self.assertEqual(report["overlap"]["verdict"], "ACTUAL_OVERLAP_UNOBSERVED")
        self.assertEqual(report["phases"], {})
        actual["periodic_numeric_polls"] = 1
        self.assertFalse(bind_execution(ordinary, actual, observation)["complete"])


if __name__ == "__main__":
    unittest.main()
