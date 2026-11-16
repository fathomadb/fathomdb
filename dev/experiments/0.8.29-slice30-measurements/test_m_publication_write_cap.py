"""Real report writes crossing the remaining allowance cannot publish qualified credit."""

from contextlib import ExitStack, redirect_stdout
import io
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

from m_runner import publish_selected, run_selected
import test_m_selected_report
from test_m_calibration import LIMITS


class PublicationWriteCapTests(unittest.TestCase):
    def setUp(self):
        self.fixture = test_m_selected_report.SelectedReportTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root = self.fixture.root
        self.base = self.fixture.fixture.base
        self.cell = self.fixture.cell
        guardian = self.root / "guardians" / "original"
        guardian.mkdir(parents=True)
        self.guardian_path = guardian / "guardian.json"
        self.guardian_path.write_text(json.dumps(self.fixture.fixture.guardian))
        self.ledger_path = self.base / "g-budget.json"
        self.ledger_path.write_text(json.dumps({"used_ns": 1000, "inflight": None,
            "selected_cell_usage": {self.cell["id"]: {"limits": LIMITS,
                "wall_ns": 898 * 10**9, "new_retained_bytes": 12345}}}))
        results = self.root / "results"
        results.mkdir()
        (results / "campaign-state.json").write_text(json.dumps([{
            "id": self.cell["id"], "cell": self.cell, "status": "complete", "complete": True}]))
        self.clock = 123000000000
        self.original_write = Path.write_text
        self.report_writes = []

    def delayed_report_write(self, path, data, *args, **kwargs):
        result = self.original_write(path, data, *args, **kwargs)
        if path.parent == self.root / "reports" and path.name.startswith("report-"):
            self.report_writes.append(path)
            self.clock += 3 * 10**9
        return result

    def test_write_overrun_revokes_returned_and_persisted_report_credit(self):
        with patch("m_runner.time.monotonic_ns", side_effect=lambda: self.clock), \
                patch.object(Path, "write_text", lambda path, data, *args, **kwargs: self.delayed_report_write(path, data, *args, **kwargs)):
            value = publish_selected(self.root)
        self.assertTrue(self.report_writes, "regression did not execute a real report write")
        self.assertNotEqual(json.loads(self.guardian_path.read_text())["exit"], 0)
        self.assertFalse(value["phase_qualified"], "write overrun left returned report qualified")
        self.assertFalse(next(row for row in value["rows"] if row["id"] == self.cell["id"])["complete"])
        saved = json.loads(self.report_writes[-1].read_text())
        self.assertFalse(saved["phase_qualified"], "write overrun left persisted report qualified")
        self.assertFalse(saved["complete"])

    def test_public_selected_run_returns_failure_after_actual_report_write_overrun(self):
        profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
            "ledger": str(self.ledger_path), "whole_cell_ns": 900 * 10**9,
            "total_shared_ns": 14400 * 10**9, "total_shared_retained_bytes": 16 * 1024**3,
            "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0,
            "other_work_cost_status": "HELD_UNAVAILABLE", "reservation_disposition": "one selected required cell"}
        manifest = json.loads((self.root / "manifest.json").read_text())
        manifest["profile"] = profile
        (self.root / "manifest.json").write_text(json.dumps(manifest))
        profile_path = self.base / "profile.json"
        profile_path.write_text(json.dumps(profile))
        options = self.base / "options.json"
        options.write_text(json.dumps({"allocation": str(self.base), "profile": str(profile_path),
                                       "cell_allowance": LIMITS}))
        original_exists = Path.exists
        def exists(path):
            return False if path == self.root else original_exists(path)
        with ExitStack() as stack:
            stack.enter_context(patch.dict(os.environ, AGENT_LONG="1", S30_TASK=str(self.base)))
            stack.enter_context(patch.object(Path, "exists", exists))
            stack.enter_context(patch("m_guard.guard", return_value={"exit": 0, "cleanup_uncertain": False}))
            stack.enter_context(patch("m_runner.retire_working_copies", return_value=[]))
            stack.enter_context(patch("m_runner.time.monotonic_ns", side_effect=lambda: self.clock))
            stack.enter_context(patch.object(Path, "write_text", lambda path, data, *args, **kwargs: self.delayed_report_write(path, data, *args, **kwargs)))
            stack.enter_context(redirect_stdout(io.StringIO()))
            value = run_selected(options, self.root, self.cell["id"], self.clock)
        self.assertTrue(self.report_writes, "public regression did not execute a real report write")
        self.assertNotEqual(value["exit"], 0, "public selected run returned success after report write overrun")


if __name__ == "__main__":
    unittest.main()
