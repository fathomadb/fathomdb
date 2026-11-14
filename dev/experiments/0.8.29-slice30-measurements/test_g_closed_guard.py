"""Actual private guardian common-work attribution with harmless nested workers."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from test_g_phase import PHASE

MODE = "closed-copy-calibration"


def phase(root, exhausted):
    for name in ("logs", "receipts"):
        (root / name).mkdir()
    allocation = root / "allocation"
    allocation.mkdir()
    prior = 1845130877940
    (allocation / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
        "used_ns": prior, "inflight": None, "cell_used_ns": {},
        "common_used_ns": {MODE: 600 * 10**9 if exhausted else 0}}))
    marker = root / "child-ran"
    worker = "\n".join([
        "import json,pathlib,sys,time",
        f"time.sleep(.08);pathlib.Path({str(marker)!r}).touch()",
        "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
    ])
    child = "\n".join([
        "import pathlib,sys",
        f"sys.path.insert(0,{str(PHASE.parent)!r})",
        "from g_closed_common import run_common",
        f"run_common(pathlib.Path({str(allocation)!r}),{MODE!r},[sys.executable,'-c',{worker!r}])",
        "import json",
        f"b=json.loads(pathlib.Path({str(allocation/'g-budget.json')!r}).read_text())",
        f"pathlib.Path({str(root/'nested-charge')!r}).write_text(str(b['common_used_ns'][{MODE!r}]))",
    ])
    harness = "\n".join([
        "import pathlib,runpy,sys",
        f"sys.path.insert(0,{str(PHASE.parent)!r})",
        "import g_resource,g_closed_guard",
        "# Synthetic resource inputs qualify lifecycle/accounting only, never host readiness.",
        "g_resource.resource_sample=lambda *_:dict(root_free=40*1024**3,task_free=300*1024**3,memory_available=20*1024**3,owned=0,measurement_retries=[])",
        f"g_closed_guard.discover=lambda _:dict(allocation={str(allocation)!r},mode={MODE!r},root={str(allocation/'frozen')!r})",
        f"sys.argv=[{str(PHASE)!r},'common-guard-control',sys.executable,'-c',{child!r}]",
        f"runpy.run_path({str(PHASE)!r},run_name='__main__')",
    ])
    completed = subprocess.run([sys.executable, '-c', harness], capture_output=True, text=True,
        timeout=15, env=dict(os.environ, S30_SOURCE=str(PHASE.parents[3]), S30_TASK=str(root)), check=False)
    candidates = list(root.rglob('common-guard-control.json'))
    report = json.loads(candidates[0].read_text()) if candidates else None
    budget = json.loads((allocation / 'g-budget.json').read_text())
    nested = int((root / "nested-charge").read_text()) if (root / "nested-charge").exists() else None
    return completed, report, budget, marker, prior, nested


class GClosedGuardianTests(unittest.TestCase):
    def test_outer_guardian_work_is_funded_without_double_charging_nested_common(self):
        with tempfile.TemporaryDirectory() as temporary:
            completed, report, budget, marker, prior, nested = phase(Path(temporary), False)
            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            self.assertTrue(marker.exists())
            self.assertGreater(budget['used_ns'] - prior, nested,
                               'outer guardian work is absent; only nested worker was charged')
            self.assertEqual(budget['used_ns'] - prior, report['closed_common']['charged_phase_ns'])
            self.assertEqual(budget['common_used_ns'][MODE], report['closed_common']['charged_phase_ns'])
            self.assertFalse(report['cleanup_uncertain'])
            self.assertFalse(report['boundary']['populated_after'])

    def test_exhausted_common_reserve_refuses_before_outer_child_starts(self):
        with tempfile.TemporaryDirectory() as temporary:
            completed, report, _budget, marker, _prior, _nested = phase(Path(temporary), True)
            self.assertNotEqual(completed.returncode, 0, 'guardian ignored exhausted common reserve')
            self.assertIsNotNone(report)
            self.assertIsNone(report['pid'])
            self.assertFalse(marker.exists())
