"""Real controller interruption must retain the active-cell charge and durable block."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from g_phase_tests import PHASE


class GClosedUncommittedTests(unittest.TestCase):
    def test_resource_stop_accounts_real_uncommitted_cell_after_private_quiescence(self):
        root = Path(tempfile.mkdtemp(prefix="g-closed-uncommitted-"))
        allocation = root / "allocation"
        allocation.mkdir()
        for name in ("logs", "receipts"):
            (root / name).mkdir()
        prior = 1845130877940
        prior_cell = 42_000_000
        (allocation / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
            "used_ns": prior, "inflight": None, "cell_used_ns": {"g-r-n100000-r1": prior_cell},
            "common_used_ns": {}, "exploratory_seed100k": {"consumed": True}}))
        ready = root / "native-ready"
        program = f"import pathlib,time;pathlib.Path({str(ready)!r}).touch();time.sleep(20)"
        controller = "\n".join([
            "import json,pathlib,sys",
            f"sys.path.insert(0,{str(PHASE.parent)!r})",
            "from g_campaign import run_g_plan",
            "from test_g_closed_modes import references",
            "from test_g_closed_plan import SPEC",
            f"run_g_plan(dict(SPEC,preparation='closed-copy-v1',closed_mode='closed-copy-read-r1'),pathlib.Path({str(allocation/'results')!r}),[sys.executable,'-c',{program!r}],allocation_root=pathlib.Path({str(allocation)!r}),reused=references())",
        ])
        harness = "\n".join([
            "import pathlib,runpy,sys",
            f"sys.path.insert(0,{str(PHASE.parent)!r})",
            "import g_resource,g_attempt,g_closed_guard",
            "actual=g_attempt.wait_unreaped",
            "g_attempt.wait_unreaped=lambda child,seconds:actual(child,.05 if seconds==30 else seconds)",
            "def sample(*_):",
            "    # Synthetic source-test observations, not host readiness.",
            f"    ready=pathlib.Path({str(ready)!r}).exists()",
            "    return dict(root_free=40*1024**3,task_free=300*1024**3,memory_available=0 if ready else 20*1024**3,owned=0,measurement_retries=[])",
            "g_resource.resource_sample=sample",
            f"g_closed_guard.discover=lambda _:dict(allocation={str(allocation)!r},mode='closed-copy-read-r1',root={str(allocation/'frozen')!r})",
            f"sys.argv=[{str(PHASE)!r},'uncommitted-control',sys.executable,'-c',{controller!r}]",
            f"runpy.run_path({str(PHASE)!r},run_name='__main__')",
        ])
        completed = subprocess.run([sys.executable, "-c", harness], capture_output=True, text=True, timeout=15,
            env=dict(os.environ, S30_SOURCE=str(PHASE.parents[3]), S30_TASK=str(root)), check=False)
        receipts = list(root.rglob("uncommitted-control.json"))
        self.assertEqual(len(receipts), 1, completed.stdout + completed.stderr)
        report = json.loads(receipts[0].read_text())
        budget = json.loads((allocation / "g-budget.json").read_text())
        self.assertTrue(ready.exists(), "fixture did not reach an actual active nested child")
        self.assertNotEqual(completed.returncode, 0)
        self.assertFalse(report["cleanup_uncertain"])
        self.assertFalse(report["boundary"]["populated_after"])
        print(json.dumps({"root": str(root), "ready": True, "used_ns": budget["used_ns"],
                          "cell_used_ns": budget["cell_used_ns"].get("g-r-n100000-r1", 0)}), flush=True)
        self.assertGreater(budget["cell_used_ns"]["g-r-n100000-r1"], prior_cell,
                           "actual active cell interval was lost when controller could not commit finally")
        self.assertIsNotNone(budget["inflight"], "interrupted uncommitted work lost its durable block")
        recovered = budget["cell_used_ns"]["g-r-n100000-r1"] - prior_cell
        self.assertEqual(recovered, report["closed_common"]["recovered_cell_ns"])
        self.assertEqual(budget["used_ns"]-prior,
                         recovered + budget["common_used_ns"]["closed-copy-read-r1"])
        self.assertEqual(budget["inflight"]["status"], "guardian_stopped")
        self.assertTrue(budget["exploratory_seed100k"]["consumed"])
