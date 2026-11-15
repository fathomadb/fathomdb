"""Harmless children with a fake boundary; these tests make no cgroup/runtime claim."""

from contextlib import ExitStack
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from m_calibration import CALIBRATION_ID
from m_guard import guard
from test_m_calibration import LIMITS


class FakeBoundary:
    def __init__(self, task):
        self.evidence = {"fixture": "no cgroup proof"}

    def create(self):
        self.procs_fd = os.open("/dev/null", os.O_WRONLY)

    def attach(self):
        os.close(self.procs_fd)

    def populated(self):
        return False

    def cleanup(self):
        os.close(self.procs_fd)


class CalibrationGuardianTests(unittest.TestCase):
    def setUp(self):
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        self.root = Path(self.stack.enter_context(tempfile.TemporaryDirectory()))
        self.ledger = self.root / "g-budget.json"
        self.ledger.write_text(json.dumps({"schema": "slice30-gd-budget/v1", "used_ns": 123,
                                          "inflight": None, "cell_used_ns": {}}))
        self.profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
            "ledger": str(self.ledger), "whole_cell_ns": 900 * 10**9,
            "total_shared_ns": 4000 * 10**9, "total_shared_retained_bytes": 10 * 1024**3,
            "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0,
            "other_work_cost_status": "HELD_UNAVAILABLE", "reservation_disposition": "control only"}
        self.stack.enter_context(patch("m_guard.TaskBoundary", FakeBoundary))
        self.stack.enter_context(patch("m_guard.resource_sample", return_value={
            "root_free": 1000 * 1024**3, "task_free": 1000 * 1024**3,
            "memory_available": 100 * 1024**3, "owned": 0}))
        self.stack.enter_context(patch("m_guard.observe_load", return_value={"load": 0.0}))

    def run_child(self, text, name, limits=LIMITS):
        return guard([sys.executable, "-c", text], allocation=self.root,
            profile=self.profile, root=self.root / name, task=self.root, calibration=limits)

    def test_preflight_refusal_is_charged_to_shared_and_calibration_allowance(self):
        with patch("m_guard.resource_sample", return_value={
                "root_free": 0, "task_free": 1000 * 1024**3,
                "memory_available": 100 * 1024**3, "owned": 0}):
            try:
                self.run_child("raise AssertionError('must not start')", "refused")
            except RuntimeError:
                pass
        value = json.loads(self.ledger.read_text())
        self.assertGreater(value["used_ns"], 123, "preflight lost its actual shared charge")
        self.assertEqual(value["calibration_usage"][CALIBRATION_ID]["wall_ns"], value["used_ns"]-123)
        self.assertGreater(value["calibration_usage"][CALIBRATION_ID]["new_retained_bytes"], 0)
        self.assertIsNone(value["m_guardian"])

    def test_busy_host_admits_d_and_m_guarded_children_and_retains_observations(self):
        for phase, calibration in (("M", LIMITS), ("M", None), ("D", None)):
            with self.subTest(phase=phase, calibration=calibration):
                profile = dict(self.profile, schema=f"slice30-{phase.lower()}-budget/v1")
                root = self.root / f"busy-{phase}-{calibration is not None}"
                marker = root / "started"
                child = f"import pathlib;pathlib.Path({str(marker)!r}).write_text('yes')"
                with patch("m_guard.observe_load", return_value={"load": 99.0}):
                    result = guard([sys.executable, "-c", child], allocation=self.root,
                        profile=profile, root=root, task=self.root, phase=phase, calibration=calibration)
                self.assertTrue(marker.exists(), result)
                self.assertEqual(result["exit"], 0)
                self.assertEqual(result["load"]["max_load"], 99.0)
                self.assertTrue(result["load"]["noisy"])
                self.assertTrue(all(row["load"]["load"] == 99.0 for row in result["resources"]))
                self.assertFalse(result["cleanup_uncertain"])

    def test_headroom_refusal_preserves_g_d_fields_and_charges_the_same_cumulative_allowance(self):
        value = json.loads(self.ledger.read_text())
        value.update(cell_used_ns={"g-prior": 101}, common_used_ns={"D": 17},
                     closed_common_roots=["original-g-root"], closed_guardian=None, d_guardian=None)
        self.ledger.write_text(json.dumps(value))
        self.profile["total_shared_ns"] = 1800 * 10**9
        with self.assertRaisesRegex(ValueError, "headroom"):
            self.run_child("raise AssertionError('must not start')", "headroom-refused")
        final = json.loads(self.ledger.read_text())
        self.assertGreater(final["used_ns"], 123, "headroom refusal silently lost its actual entry charge")
        self.assertEqual(final["calibration_usage"][CALIBRATION_ID]["wall_ns"], final["used_ns"]-123)
        self.assertEqual(final["schema"], "slice30-gd-budget/v1")
        self.assertEqual(final["cell_used_ns"], {"g-prior": 101})
        self.assertEqual(final["common_used_ns"]["D"], 17)
        self.assertEqual(final["closed_common_roots"], ["original-g-root"])
        self.assertIsNone(final.get("m_guardian"))

    def test_whole_phase_deadline_stops_child_without_active_cell(self):
        marker = self.root / "deadline" / "started"
        real_clock = time.monotonic_ns
        began = real_clock()

        def clock():
            return began + 881 * 10**9 if marker.exists() else real_clock()

        child = f"import pathlib,time;pathlib.Path({str(marker)!r}).write_text('yes');time.sleep(.3)"
        with patch("m_guard.time.monotonic_ns", side_effect=clock):
            result = self.run_child(child, "deadline")
        self.assertTrue(marker.exists())
        self.assertNotEqual(result["exit"], 0, "whole calibration phase deadline was ignored")
        self.assertEqual(result.get("stop"), "calibration phase cleanup reserve")
        self.assertFalse(result["cleanup_uncertain"])

    def test_growth_stop_retains_failed_bytes_and_blocks_renewal_under_another_root(self):
        limits = dict(LIMITS, new_retained_bytes=16384)
        raw = self.root / "growth" / "failed.raw"
        child = f"import pathlib,time;pathlib.Path({str(raw)!r}).write_bytes(b'x'*32768);time.sleep(.3)"
        result = self.run_child(child, "growth", limits)
        self.assertNotEqual(result["exit"], 0, "calibration retained-growth stop was ignored")
        self.assertEqual(result.get("stop"), "calibration retained growth")
        value = json.loads(self.ledger.read_text())
        usage = value["calibration_usage"][CALIBRATION_ID]
        self.assertGreaterEqual(usage["new_retained_bytes"], 32768)
        self.assertTrue(raw.exists())
        with self.assertRaisesRegex(ValueError, "remaining"):
            self.run_child("raise AssertionError('must not start')", "new-root", limits)

    def test_qualification_failures_accumulate_actual_charges_and_artifacts_across_roots(self):
        prior = 123
        for name in ("first-failure", "other-root"):
            raw = self.root / name / "qualification.raw"
            result = self.run_child(f"import pathlib;pathlib.Path({str(raw)!r}).write_text('retained');exit(7)", name)
            self.assertEqual(result["exit"], 7)
            value = json.loads(self.ledger.read_text())
            self.assertIn("calibration_usage", value, "failed qualification lost calibration usage")
            self.assertGreater(value["used_ns"], prior)
            self.assertEqual(value["calibration_usage"][CALIBRATION_ID]["wall_ns"], value["used_ns"]-123)
            self.assertTrue(raw.exists())
            prior = value["used_ns"]


if __name__ == "__main__":
    unittest.main()
