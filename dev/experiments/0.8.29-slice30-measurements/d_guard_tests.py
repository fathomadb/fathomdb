"""Explicit delegated live D guardian controls with harmless controllers only."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

from d_runner import run_guarded


class DGuardianTests(unittest.TestCase):
    def fixture(self, allocation):
        """State an isolated source-control allocation, never a scientific profile."""
        ledger = allocation/"g-budget.json"
        budget = {"schema": "slice30-gd-budget/v1", "used_ns": 123,
                  "inflight": None, "cell_used_ns": {"g-prior": 101},
                  "common_used_ns": {"M": 17}, "m_guardian": None}
        ledger.write_text(json.dumps(budget))
        profile = {"schema": "slice30-d-budget/v1", "status": "APPROVED",
                   "ledger": str(ledger), "whole_cell_ns": 900*10**9,
                   "total_shared_ns": 60*10**9, "total_shared_retained_bytes": 1024**3,
                   "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0,
                   "scope": "isolated-harmless-source-control"}
        return ledger, budget, profile

    def test_actual_controller_death_recovers_d_cell_and_cleans_detached_child(self):
        task = Path(os.environ["S30_TASK"])
        with tempfile.TemporaryDirectory(prefix="d-guardian-", dir=task/"tmp") as temporary:
            allocation = Path(temporary)
            ledger, _, profile = self.fixture(allocation)
            ready, child_ready = allocation/"controller-started", allocation/"child-started"
            controller = "\n".join([
                "import json,os,pathlib,signal,subprocess,sys,time",
                f"sys.path.insert(0,{str(Path(__file__).parent)!r})",
                "from m_guard import write_budget",
                f"p=pathlib.Path({str(ledger)!r}); b=json.loads(p.read_text())",
                "assert b['d_guardian'] and b['m_guardian'] is None",
                "b['inflight']={'cell':'D-source-interrupted','started_monotonic_ns':time.monotonic_ns()}",
                "write_budget(p,b)",
                f"code='import os,pathlib,time;pathlib.Path('+repr({str(child_ready)!r})+').write_text(str(os.getpid()));time.sleep(60)'",
                "subprocess.Popen([sys.executable,'-c',code],start_new_session=True)",
                "until=time.monotonic()+2",
                f"while not pathlib.Path({str(child_ready)!r}).exists() and time.monotonic()<until: time.sleep(0.005)",
                f"pathlib.Path({str(ready)!r}).write_text('started')",
                "time.sleep(0.05)",
                "os.kill(os.getpid(),signal.SIGTERM)",
            ])
            result = run_guarded([sys.executable, "-c", controller], allocation=allocation,
                                 profile=profile, root=allocation/"results", task=task)
            self.assertTrue(ready.exists(), "D guarded entry never started its controller")
            self.assertTrue(child_ready.exists(), "D control never established its detached owned child")
            self.assertFalse(result["cleanup_uncertain"])
            self.assertTrue(result["boundary"]["removed_exact_empty_boundary"])
            child_stat = Path(f"/proc/{int(child_ready.read_text())}/stat")
            if child_stat.exists():
                self.assertIn(child_stat.read_text().rsplit(") ", 1)[1].split()[0], ("Z", "X"))
            final = json.loads(ledger.read_text())
            self.assertGreater(final["cell_used_ns"]["D-source-interrupted"], 0)
            self.assertGreater(final["common_used_ns"]["D"], 0)
            self.assertEqual(final["used_ns"]-123,
                final["cell_used_ns"]["D-source-interrupted"]+final["common_used_ns"]["D"])
            self.assertEqual(final["common_used_ns"]["M"], 17)
            self.assertEqual(final["cell_used_ns"]["g-prior"], 101)
            self.assertEqual(final["inflight"]["status"], "guardian_stopped")
            self.assertIsNone(final["d_guardian"])
            self.assertIsNone(final["m_guardian"])

    def test_completed_actual_cell_and_d_common_reconcile_outer_wall_once(self):
        task = Path(os.environ["S30_TASK"])
        with tempfile.TemporaryDirectory(prefix="d-guardian-", dir=task/"tmp") as temporary:
            allocation = Path(temporary)
            ledger, _, profile = self.fixture(allocation)
            ready = allocation/"controller-started"
            controller = "\n".join([
                "import json,pathlib,sys,time",
                f"sys.path.insert(0,{str(Path(__file__).parent)!r})",
                "from m_guard import write_budget",
                f"p=pathlib.Path({str(ledger)!r}); b=json.loads(p.read_text())",
                "assert b['d_guardian'] and b['m_guardian'] is None",
                f"pathlib.Path({str(ready)!r}).write_text('started')",
                "start=time.monotonic_ns(); time.sleep(0.02); elapsed=time.monotonic_ns()-start",
                "b['used_ns']+=elapsed; b['cell_used_ns']['D-source-complete']=elapsed",
                "write_budget(p,b)",
            ])
            result = run_guarded([sys.executable, "-c", controller], allocation=allocation,
                                 profile=profile, root=allocation/"results", task=task)
            self.assertTrue(ready.exists(), "D common accounting never owned actual controller work")
            self.assertEqual(result["exit"], 0)
            self.assertFalse(result["cleanup_uncertain"])
            final = json.loads(ledger.read_text())
            self.assertGreater(final["cell_used_ns"]["D-source-complete"], 0)
            self.assertEqual(final["used_ns"]-123,
                final["cell_used_ns"]["D-source-complete"]+final["common_used_ns"]["D"])
            self.assertEqual(final["common_used_ns"]["M"], 17)
            self.assertIsNone(final["d_guardian"])

    def test_unknown_or_exhausting_other_work_and_retained_custody_refuse_start(self):
        task = Path(os.environ["S30_TASK"])
        for refusal in ("unknown-time", "unknown-bytes", "reserved-time", "reserved-bytes",
                        "inflight", "m_guardian", "closed_guardian", "d_guardian"):
            with self.subTest(refusal=refusal), tempfile.TemporaryDirectory(prefix="d-refusal-", dir=task/"tmp") as temporary:
                allocation = Path(temporary)
                ledger, budget, profile = self.fixture(allocation)
                ready = allocation/"forbidden-start"
                if refusal == "unknown-time":
                    profile["reserved_other_work_ns"] = None
                elif refusal == "unknown-bytes":
                    del profile["reserved_other_work_bytes"]
                elif refusal == "reserved-time":
                    profile["reserved_other_work_ns"] = 60*10**9
                elif refusal == "reserved-bytes":
                    profile["reserved_other_work_bytes"] = 1024**3
                else:
                    budget[refusal] = {"status": "unresolved-source-control"}
                    ledger.write_text(json.dumps(budget))
                result = run_guarded([sys.executable, "-c",
                    f"import pathlib;pathlib.Path({str(ready)!r}).write_text('started')"],
                    allocation=allocation, profile=profile, root=allocation/"results", task=task)
                self.assertFalse(ready.exists())
                self.assertNotEqual(result["exit"], 0)
                self.assertEqual(json.loads(ledger.read_text()), budget)


if __name__ == "__main__":
    unittest.main()
