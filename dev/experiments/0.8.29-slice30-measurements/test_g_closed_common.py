"""Actual common-worker wall charging and nonrenewable reserve admission."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from g_closed_common import run_common

MODE = "closed-copy-calibration"


def worker(marker, delay=.03):
    return [sys.executable, "-c", "\n".join([
        "import json,pathlib,sys,time",
        f"time.sleep({delay!r});pathlib.Path({str(marker)!r}).touch()",
        "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
    ])]


class GClosedCommonTests(unittest.TestCase):
    def test_real_common_work_is_charged_once_to_shared_and_named_reserve(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prior = 1845130877940
            (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": prior, "inflight": None, "cell_used_ns": {}, "common_used_ns": {}}))
            outcome = run_common(root, MODE, worker(root / "ran"))
            self.assertEqual(outcome["status"], "complete")
            budget = json.loads((root / "g-budget.json").read_text())
            self.assertGreater(budget["used_ns"], prior, "real common worker time disappeared from shared allocation")
            charged = budget["used_ns"] - prior
            self.assertEqual(budget["common_used_ns"][MODE], charged)
            self.assertGreaterEqual(charged, 30_000_000)
            self.assertIsNone(budget["inflight"])
            self.assertEqual(budget["cell_used_ns"], {}, "common time was double charged to a primary cell")

    def test_exhausted_common_reserve_refuses_a_new_root_before_child_start(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 2000 * 10**9, "inflight": None, "cell_used_ns": {},
                "common_used_ns": {MODE: 600 * 10**9}}))
            marker = root / "forbidden"
            outcome = run_common(root, MODE, worker(marker))
            self.assertFalse(marker.exists(), "new common operation renewed the600s reserve")
            self.assertEqual(outcome["status"], "not_run_cap")
            self.assertEqual(outcome["reason"], "common_time_exhausted")
