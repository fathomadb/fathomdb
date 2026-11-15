"""Fake-dispatch controls assert selection/admission/reporting, never database behavior."""

from contextlib import ExitStack
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from m_calibration import CALIBRATION_ID, CELL_ID, control, preparation_bound, selected_cell
from m_controller import load_plan
from m_runner import main, report
from test_m_calibration import LIMITS


class CalibrationEntryTests(unittest.TestCase):
    def setUp(self):
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        self.root = Path(self.stack.enter_context(tempfile.TemporaryDirectory()))
        self.query_file = self.root / "queries.jsonl"
        self.query_file.write_text('"alpha"\n' * 500)

    def manifest(self):
        return {"schema": "slice30-m-calibration/v1", "scope": "bounded-natural-calibration",
            "uuid": "literal-calibration", "source_sha": "actual-subject-in-runtime",
            "root": str(self.root), "allocation": str(self.root), "task": str(self.root),
            "plan": [selected_cell(CELL_ID)], "calibration": LIMITS,
            "package": [{"bytes": 715223040}], "inputs": {"queries": {"path": str(self.query_file)}},
            "profile": {"total_shared_ns": 4000 * 10**9, "reserved_other_work_ns": 0,
                        "total_shared_retained_bytes": 10 * 1024**3}}

    def test_public_command_uses_original_approved_ledger_and_one_guarded_stage(self):
        profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
            "ledger": str(self.root / "g-budget.json"), "whole_cell_ns": 900 * 10**9,
            "total_shared_ns": 14400 * 10**9, "total_shared_retained_bytes": 16 * 1024**3,
            "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0,
            "other_work_cost_status": "HELD_UNAVAILABLE", "reservation_disposition": "M calibration only"}
        profile_path = self.root / "profile.json"
        profile_path.write_text(json.dumps(profile))
        options = self.root / "options.json"
        options.write_text(json.dumps({"allocation": str(self.root), "profile": str(profile_path),
                                       "calibration": LIMITS}))
        argv = ["m_runner.py", "calibrate", "--options", str(options),
                "--root", str(self.root / "selected"), "--cell", CELL_ID]
        with patch.dict(os.environ, AGENT_LONG="1", S30_TASK=str(self.root)), patch("sys.argv", argv), \
                patch("m_guard.guard", return_value={"exit": 0, "cleanup_uncertain": False}) as guarded:
            self.assertEqual(main(), 0, "public bounded calibration stage did not dispatch")
        self.assertEqual(guarded.call_count, 1)
        args, kwargs = guarded.call_args
        self.assertIn("_calibrate", args[0])
        self.assertEqual(kwargs["profile"], profile)
        self.assertEqual(kwargs["allocation"], str(self.root))
        self.assertEqual(kwargs["calibration"], LIMITS)
        self.assertEqual(kwargs["common_limit_ns"], 100 * 10**9)

    def test_dispatch_reuses_exact_selected_prepare_and_payload_with_positive_deadline(self):
        manifest = self.manifest()
        budget = {"schema": "slice30-gd-budget/v1", "used_ns": 123, "cell_used_ns": {},
                  "inflight": None, "m_guardian": {"started_monotonic_ns": __import__("time").monotonic_ns()},
                  "calibration_usage": {CALIBRATION_ID: {"limits": LIMITS, "wall_ns": 0,
                                                          "new_retained_bytes": 0}}}
        ledger = self.root / "g-budget.json"
        ledger.write_text(json.dumps(budget))
        calls = []

        def attempt(cell, root, command, *, deadline_ns, prepare, authorize, monitor, register):
            path = Path(root) / CALIBRATION_ID / "attempt-0001"
            path.mkdir(parents=True)
            register(cell, path)
            self.assertIsNone(authorize(False))
            prepare(cell, path, deadline_ns)
            self.assertIsNone(authorize(True))
            self.assertIsNone(monitor())
            calls.append(command(cell, path, deadline_ns))
            return {"id": cell["id"], "cell": cell, "attempt": str(path), "complete": True,
                    "status": "complete", "cleanup_uncertain": False, "live_owned_pids": []}

        with patch("m_runtime.prepare") as prepared, patch("m_calibration.run_attempt", attempt, create=True), \
                patch("m_calibration.observe_load", return_value={"load": 99.0}, create=True):
            result = control(self.root, budget["m_guardian"], manifest)
        self.assertIsInstance(result, dict, "selected calibration controller did not execute")
        self.assertTrue(result["complete"])
        self.assertEqual(prepared.call_count, 1)
        self.assertEqual(prepared.call_args.args[0], manifest)
        self.assertEqual(prepared.call_args.args[1]["id"], CALIBRATION_ID)
        self.assertEqual(prepared.call_args.args[1]["timed"], 1000)
        self.assertEqual(len(calls), 1)
        self.assertIn("_cell", calls[0])
        final = json.loads(ledger.read_text())
        self.assertGreater(final["used_ns"], 123)
        self.assertIsNone(final["inflight"])
        self.assertEqual(set(final["cell_used_ns"]), {CALIBRATION_ID})

    def test_closed_copy_and_positive_output_bound_refuse_before_preparation(self):
        manifest = self.manifest()
        bound = preparation_bound(manifest)
        self.assertIsInstance(bound, int, "closed package plus output bound is missing")
        self.assertGreater(bound, 715223040 + 1024**3)
        self.assertLess(bound, 2 * 1024**3)

    def test_calibration_success_keeps_all46_primary_rows_held(self):
        manifest = self.manifest()
        (self.root / "manifest.json").write_text(json.dumps(manifest))
        (self.root / "results").mkdir()
        (self.root / "results/campaign-state.json").write_text(json.dumps([
            {"id": CALIBRATION_ID, "status": "complete", "complete": True}]))
        value = report(self.root)
        self.assertEqual(len(value["rows"]), 46, "calibration replaced primary46 report obligations")
        self.assertEqual([row["id"] for row in value["rows"]], [row["id"] for row in load_plan()])
        self.assertFalse(value["complete"])
        self.assertTrue(all(row["status"] == "held" for row in value["rows"]))
        self.assertTrue(value["calibration_complete"])
        self.assertEqual(value["calibrations"][0]["id"], CALIBRATION_ID)
        self.assertIsNone(value["verdict"])
        self.assertIsNone(value["calibrated_threshold"])


if __name__ == "__main__":
    unittest.main()
