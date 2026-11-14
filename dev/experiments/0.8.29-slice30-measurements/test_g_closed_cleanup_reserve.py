"""Real graceful cleanup must fit the declared whole-cell remaining allowance."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_campaign
from test_g_closed_modes import references
from test_g_closed_plan import SPEC


class GClosedCleanupReserveTests(unittest.TestCase):
    def test_real_child_cleanup_is_reserved_before_execution_deadline(self):
        root = Path(tempfile.mkdtemp(prefix="g-closed-cleanup-clock-"))
        ready = root / "ready"
        prior = 1500*10**9 - 600_000_000
        (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
            "used_ns": 2000*10**9, "inflight": None, "cell_used_ns": {"g-r-n100000-r1": prior}}))
        child = "\n".join([
            "import pathlib,signal,sys,time",
            "def stop(*_):time.sleep(.1);sys.exit(0)",
            "signal.signal(signal.SIGTERM,stop)",
            f"pathlib.Path({str(ready)!r}).touch()",
            "time.sleep(20)",
        ])
        spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-read-r1")
        # Scaled admission reserve only; real WNOWAIT, signals and cleanup remain unchanged.
        with patch.object(g_campaign, "CLOSED_CLEANUP_RESERVE_NS", 300_000_000):
            rows = g_campaign.run_g_plan(spec, root / "results", [sys.executable, "-c", child],
                                        allocation_root=root, reused=references())
        row = next(row for row in rows if row["id"] == "g-r-n100000-r1")
        budget = json.loads((root / "g-budget.json").read_text())
        self.assertTrue(ready.exists(), "clock fixture never launched its real owned child")
        self.assertEqual(row["status"], "failed")
        self.assertIn("timeout", row["error"])
        self.assertFalse(row["cleanup_uncertain"])
        self.assertIsNone(budget["inflight"])
        print(json.dumps({"root": str(root), "ready": True,
                          "charged_ns": budget["cell_used_ns"][row["id"]]-prior}), flush=True)
        self.assertLessEqual(budget["cell_used_ns"][row["id"]], 1500*10**9,
                             "cleanup began after execution consumed the whole remaining cell cap")
