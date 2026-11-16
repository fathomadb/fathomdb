"""Actual harmless child checks the selected D clock; fake boundary makes no cgroup claim."""

from contextlib import ExitStack, redirect_stdout
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from d_runner import main
from m_guard import guard
from test_m_calibration_guard import FakeBoundary

SECOND = 10**9


class DSelectedGuardTests(unittest.TestCase):
    def setUp(self):
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        self.root = Path(self.stack.enter_context(tempfile.TemporaryDirectory()))
        self.ledger = self.root / "g-budget.json"
        self.ledger.write_text(json.dumps({"schema": "slice30-gd-budget/v1", "used_ns": 100 * SECOND,
                                          "inflight": None, "cell_used_ns": {}}))
        self.profile = {"schema": "slice30-d-budget/v1", "status": "APPROVED",
            "ledger": str(self.ledger), "whole_cell_ns": 1500 * SECOND,
            "total_shared_ns": 14400 * SECOND, "total_shared_retained_bytes": 16 * 1024**3,
            "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0}

    def test_selected1500s_guard_keeps_actual_child_at1000s_and_charges_same_ledger(self):
        marker = self.root / "started"
        code = "\n".join([
            "import json,os,pathlib,time", f"p=pathlib.Path({str(self.ledger)!r})",
            "v=json.loads(p.read_text())",
            "v['inflight']={'cell':'d-c-held-n100000-r0','started_monotonic_ns':v['d_guardian']['started_monotonic_ns']}",
            "q=p.with_suffix('.child-tmp');q.write_text(json.dumps(v));os.replace(q,p)",
            f"pathlib.Path({str(marker)!r}).write_text('yes')", "time.sleep(.25)"])
        real_clock = time.monotonic_ns
        began = real_clock()
        def clock():
            return began + 1000 * SECOND if marker.exists() else real_clock()
        with patch("m_guard.TaskBoundary", FakeBoundary), patch("m_guard.resource_sample", return_value={
                "root_free": 1000 * 1024**3, "task_free": 1000 * 1024**3,
                "memory_available": 100 * 1024**3, "owned": 0}), patch("m_guard.time.monotonic_ns", side_effect=clock):
            try:
                result = guard([sys.executable, "-c", code], allocation=self.root, profile=self.profile,
                               root=self.root / "stage", task=self.root, phase="D", started_ns=began)
            except ValueError:
                result = {"exit": 125}
        self.assertTrue(marker.exists(), "selected1500s D guard refused before actual harmless child")
        self.assertEqual(result["exit"], 0, "obsolete900s guardian stopped selected1500s cell")
        value = json.loads(self.ledger.read_text())
        self.assertEqual(value["cell_used_ns"]["d-c-held-n100000-r0"], 1000 * SECOND)
        self.assertEqual(value["used_ns"], 1100 * SECOND)
        self.assertFalse(result["cleanup_uncertain"])

    def public(self, whole_seconds):
        profile = self.root / "profile.json"
        profile.write_text(json.dumps(dict(self.profile, whole_cell_ns=whole_seconds * SECOND)))
        review = self.root / "review.json"
        review.write_text(json.dumps({"n": 100000, "mode": "held", "verdict": "ACCEPT"}))
        native = self.root / "unexecuted-native"
        native.write_text('literal native metadata fixture')
        native.chmod(0o700)
        options = self.root / "options.json"
        options.write_text(json.dumps({"root": str(self.root / "selected"), "allocation": str(self.root),
            "profile": str(profile), "binary": str(native), "initial": False,
            "common_limit_ns": 60 * SECOND, "review": str(review)}))
        argv = ["d_runner.py", "run", "--options", str(options)]
        with patch.dict(os.environ, AGENT_LONG="1", S30_TASK=str(self.root)), patch("sys.argv", argv), \
                patch("d_runner.run_guarded", return_value={"exit": 125, "cleanup_uncertain": False,
                      "charged_ns": None}) as selected, redirect_stdout(io.StringIO()):
            try:
                main()
                refused = False
            except ValueError:
                refused = True
        return refused, selected

    def test_public100k_stage_refuses_mismatched900s_profile_before_dispatch(self):
        refused, selected = self.public(900)
        self.assertTrue(refused, "100k public entry accepted a900s outer clock against1500s cell")
        self.assertEqual(selected.call_count, 0)

    def test_public100k_stage_passes_selected1500s_profile_to_existing_guard(self):
        refused, selected = self.public(1500)
        self.assertFalse(refused)
        self.assertEqual(selected.call_count, 1)
        self.assertEqual(selected.call_args.kwargs["profile"]["whole_cell_ns"], 1500 * SECOND)


if __name__ == "__main__":
    unittest.main()
