"""Current native C JSONL hydration through the connected report/accounting route."""

import copy
import json
from pathlib import Path
import tempfile
import unittest

from d_driver import _qualify_native
from test_d_report import literal_execution


def write_rows(path, rows):
    """Retain explicit unit frames in the native capture file layout."""
    path.write_text("".join(json.dumps(row)+"\n" for row in rows))


def stored_case(directory):
    """Use the accepted hand two-group fixture, not a scientific D result."""
    identity, actual, observed = literal_execution()
    base = {"cell": identity, "engine_id": 42, "session_id": 7,
            "started_ns": 0, "ended_ns": 1}
    state = {"scheduler_runtime_threads": 2, "embedder_pool_size": 5, "admission_capacity": 128,
             "active_jobs": 0, "active_scans": 0, "queued_jobs": 0, "in_flight_count": 0,
             "frozen": False, "pending_scan": False, "stopping": False}
    paths = {name: directory / (name+".jsonl") for name in ("records", "runtime", "pending", "wal")}
    write_rows(paths["records"], observed["records"])
    write_rows(paths["runtime"], [{**base, "state": state},
        {**base, "started_ns": 90, "ended_ns": 91, "state": state}])
    write_rows(paths["pending"], [{**base, "pending_count": 0}, *observed["pending"]])
    write_rows(paths["wal"], [{**base, "db_bytes": 4096, "wal_exists": True, "wal_bytes": 64,
        "frame_count": None, "checkpoint_status": None},
        {**base, "started_ns": 90, "ended_ns": 91, "db_bytes": 4096,
         "wal_exists": True, "wal_bytes": 128, "frame_count": None, "checkpoint_status": None}])
    metadata = {key: observed[key] for key in ("engine_id", "session_id", "dropped_records", "issues")}
    metadata.update({name+"_path": str(path) for name, path in paths.items()})
    metadata.update(runtime_samples=2, readiness_samples=2, wal_samples=2,
                    helpers_joined_ns=49900, final_take_after_close=True)
    actual.update(observation=metadata, closed_ns=50000)
    cell = {"id": identity["cell_id"], **{key: identity[key] for key in ("n", "mode", "layer", "repeat")}}
    result = {"status": "complete", "attempt_id": "one", "native": actual, "peak_bytes": 256*1024**2,
              "elapsed_ns": 1, "cleanup_uncertain": False, "live_owned_pids": [], "unit_diagnostic": True}
    return cell, result, paths


class DRetainedStreamTests(unittest.TestCase):
    def test_actual_capture_paths_feed_current_group_and_overlap_qualification(self):
        with tempfile.TemporaryDirectory() as temporary:
            cell, result, _ = stored_case(Path(temporary))
            before = copy.deepcopy(result["native"]["observation"])
            routed = _qualify_native(cell, result)
            self.assertEqual(routed["status"], "complete", "retained native C streams never reached report binding")
            self.assertTrue(routed["observations_complete"])
            self.assertEqual(routed["execution_report"]["committed_success_rows"], 2)
            self.assertEqual(routed["execution_report"]["overlap"]["operation_id"], 17)
            self.assertEqual(routed["native"]["observation"], before, "hydration mutated the retained native header")
            self.assertNotIn("records", routed["native"]["observation"])
            json.dumps(routed)

    def test_missing_or_partial_required_stream_remains_unavailable_without_rewriting_prefix(self):
        for defect in ("missing_wal", "partial_record"):
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as temporary:
                cell, result, paths = stored_case(Path(temporary))
                if defect == "missing_wal":
                    paths["wal"].unlink()
                else:
                    paths["records"].write_text('{"cell":')
                routed = _qualify_native(cell, result)
                self.assertEqual(routed["status"], "incomplete")
                self.assertIn("retained_stream_unavailable", routed["execution_report"]["issues"])
                self.assertIsNone(routed["execution_report"]["committed_success_rows"])
                if defect == "partial_record":
                    self.assertEqual(paths["records"].read_text(), '{"cell":')

    def test_unknown_runtime_sample_cannot_be_replaced_by_successful_native_header(self):
        with tempfile.TemporaryDirectory() as temporary:
            cell, result, paths = stored_case(Path(temporary))
            write_rows(paths["runtime"], [{"cell": result["native"]["cell"], "engine_id": 42,
                "session_id": 7, "started_ns": 0, "ended_ns": 1, "state": None,
                "error": "literal state unavailable"}])
            routed = _qualify_native(cell, result)
            self.assertEqual(routed["status"], "incomplete")
            self.assertFalse(routed["observations_complete"])
            self.assertIn("runtime_sample_unavailable", routed["execution_report"]["issues"])

    def test_t_does_not_open_or_borrow_any_characterization_stream(self):
        with tempfile.TemporaryDirectory() as temporary:
            cell, result, paths = stored_case(Path(temporary))
            for path in paths.values():
                path.unlink()
            ordinary = {**result["native"]["cell"], "cell_id": "tiny-t-overlap-n2-r0", "layer": "T", "attempt_id": "two"}
            cell.update(id=ordinary["cell_id"], layer="T")
            result.update(attempt_id="two")
            result["native"].update(cell=ordinary, collection_enabled=False,
                periodic_numeric_polls=0, periodic_readiness_polls=0)
            routed = _qualify_native(cell, result)
            self.assertEqual(routed["status"], "complete")
            self.assertIsNone(routed["observations_complete"])
            self.assertIsNone(routed["execution_report"]["committed_success_rows"])
            self.assertEqual(routed["execution_report"]["overlap"]["verdict"], "ACTUAL_OVERLAP_UNOBSERVED")


if __name__ == "__main__":
    unittest.main()
