"""Public closed reports consume the common reserve before returning any output."""

from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_closed_entry
import g_runner

MODE = "closed-copy-calibration"


class GClosedPublicReportTests(unittest.TestCase):
    def test_actual_report_worker_is_charged_and_exhausted_reserve_refuses_output(self):
        for consumed in (False, True):
            with self.subTest(consumed=consumed), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                frozen = root / "frozen"
                frozen.mkdir()
                (frozen / "manifest.json").write_text(json.dumps({"plan": [], "allocation": str(root),
                    "preparation": "closed-copy-v1", "stage": MODE}))
                initial = 600 * 10**9 if consumed else 0
                (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                    "used_ns": 1845130877940, "inflight": None, "common_used_ns": {MODE: initial}}))
                marker = root / "report-worker"
                program = [sys.executable, "-c", "\n".join([
                    "import json,pathlib,sys,time",
                    f"time.sleep(.06);pathlib.Path({str(marker)!r}).touch()",
                    f"root=pathlib.Path({str(frozen)!r})",
                    "(root/'summary-control.json').write_text('{}')",
                    "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps(dict(complete=True,value=dict(cells=[],complete=False))))",
                ])]
                failure = None
                with patch.dict(os.environ, {"S30_TASK": str(root)}), \
                    patch("g_boundary.require_known_containment"), \
                        patch.object(g_closed_entry, "worker_command", return_value=program), \
                        patch.object(sys, "argv", [str(Path(g_runner.__file__)), "report", str(frozen)]), \
                        redirect_stdout(io.StringIO()):
                    try:
                        g_runner.main()
                    except (RuntimeError, SystemExit) as error:
                        failure = error
                budget = json.loads((root / "g-budget.json").read_text())
                if consumed:
                    self.assertFalse(list(frozen.glob("summary*.json")), "exhausted common reserve still published a report")
                    self.assertFalse(marker.exists())
                    self.assertIn("common_time_exhausted", str(failure))
                else:
                    self.assertGreaterEqual(budget["common_used_ns"][MODE], 60_000_000,
                        "public report returned without charging its blocking worker")
                    self.assertTrue(marker.exists())
                    self.assertEqual(budget["used_ns"]-1845130877940, budget["common_used_ns"][MODE])
                    self.assertIsNone(budget["inflight"])
                    self.assertIsInstance(failure, SystemExit)
                    self.assertEqual(failure.code, 1, "partial report implied whole G completion")
