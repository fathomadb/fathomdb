"""A real controller/nested child failure retains exact actual shared time."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

from m_guard import guard


class GuardianAccountingTests(unittest.TestCase):
    """The guardian remains outside the killed controller's owned boundary."""

    def test_controller_death_recovers_active_cell_and_keeps_block(self):
        task = Path(os.environ["S30_TASK"])
        with tempfile.TemporaryDirectory(prefix="m-guardian-", dir=task / "tmp") as temporary:
            allocation = Path(temporary)
            ledger = allocation / "g-budget.json"
            ledger.write_text(json.dumps({"schema": "slice30-gd-budget/v1", "used_ns": 123,
                                          "inflight": None, "cell_used_ns": {}}))
            controller = """
import json,os,pathlib,signal,subprocess,sys,time
p=pathlib.Path(sys.argv[1]); b=json.loads(p.read_text())
b['inflight']={'cell':'M-source-interrupted','started_monotonic_ns':time.monotonic_ns()}
p.write_text(json.dumps(b))
child=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)'],start_new_session=True)
time.sleep(.1)
os.kill(os.getpid(),signal.SIGTERM)
"""
            profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
                       "ledger": str(ledger), "whole_cell_ns": 900 * 10**9,
                       "total_shared_ns": 60 * 10**9, "total_shared_retained_bytes": 1024**3}
            result = guard([sys.executable, "-c", controller, str(ledger)], allocation=allocation,
                           profile=profile, root=allocation / "results", task=task)
            self.assertFalse(result["cleanup_uncertain"])
            self.assertTrue(result["boundary"]["removed_exact_empty_boundary"])
            final = json.loads(ledger.read_text())
            self.assertGreater(final["cell_used_ns"].get("M-source-interrupted", 0), 0,
                               "active cell time was misclassified or lost after controller death")
            self.assertEqual(final["inflight"]["status"], "guardian_stopped")
            self.assertEqual(final["used_ns"] - 123,
                             final["cell_used_ns"]["M-source-interrupted"] + final["common_used_ns"]["M"])
            self.assertIsNone(final["m_guardian"])


if __name__ == "__main__":
    unittest.main()
