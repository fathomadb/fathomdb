"""Prospective1500s limits retain prior charges and actual absolute deadlines."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_campaign import run_g_plan
from test_g_closed_modes import harmless, references
from test_g_closed_plan import SPEC


class GClosedAllowanceTests(unittest.TestCase):
    def test_charge_above_old_900_seconds_does_not_erase_new_remaining_allowance(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prior = 900 * 10**9
            (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 2000 * 10**9, "inflight": None,
                "cell_used_ns": {"g-r-n100000-r1": prior}}))
            calls = root / "calls"
            spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-read-r1")
            rows = run_g_plan(spec, root / "results", harmless(calls), allocation_root=root,
                              reused=references())
            row = next(row for row in rows if row["id"] == "g-r-n100000-r1")
            self.assertEqual(row["status"], "complete", "controller still enforces historical900s")
            self.assertEqual(calls.read_text().splitlines(), ["g-r-n100000-r1"])
            budget = json.loads((root / "g-budget.json").read_text())
            self.assertGreater(budget["cell_used_ns"][row["id"]], prior)
            self.assertGreater(budget["used_ns"], 2000 * 10**9)
            self.assertIsNone(budget["inflight"])

    def test_prior_charge_leaves_only_50ms_and_terminates_real_child(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prior = 1500 * 10**9 - 50_000_000
            (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 2000 * 10**9, "inflight": None,
                "cell_used_ns": {"g-r-n100000-r1": prior}}))
            late = root / "forbidden"
            command = [sys.executable, "-c", f"import pathlib,time;time.sleep(.4);pathlib.Path({str(late)!r}).touch()"]
            spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-read-r1")
            started = time.monotonic()
            rows = run_g_plan(spec, root / "results", command, allocation_root=root,
                              reused=references())
            row = next(row for row in rows if row["id"] == "g-r-n100000-r1")
            self.assertEqual(row["status"], "failed", "remaining allowance was refused at obsolete900s")
            self.assertIn("timeout", row["error"])
            self.assertFalse(row["cleanup_uncertain"])
            self.assertLess(time.monotonic() - started, 2)
            time.sleep(.45)
            self.assertFalse(late.exists(), "timed-out owned child survived its deadline")
