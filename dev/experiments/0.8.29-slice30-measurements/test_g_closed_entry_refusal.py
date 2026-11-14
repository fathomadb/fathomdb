"""Failed common entry still retains its actual observed work without starting a payload."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

PHASE = Path(__file__).with_name("g_phase.py").resolve()


class GClosedEntryRefusalTests(unittest.TestCase):
    def test_floor_and_consumed_reserve_refusals_charge_actual_entry_without_renewal(self):
        for failure in ("floor", "common"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                allocation = root / "allocation"
                allocation.mkdir()
                for name in ("logs", "receipts"):
                    (root / name).mkdir()
                mode = "closed-copy-calibration"
                prior = 1845130877940
                common = 600*10**9 if failure == "common" else 0
                token = {"consumed": True, "original": "preserved"}
                (allocation / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                    "used_ns": prior, "inflight": None, "common_used_ns": {mode: common},
                    "cell_used_ns": {"g1-n100000": 657935962544}, "exploratory_seed100k": token}))
                marker = root / "forbidden-payload"
                payload = f"import pathlib;pathlib.Path({str(marker)!r}).touch()"
                harness = "\n".join([
                    "import runpy,sys,time",
                    f"sys.path.insert(0,{str(PHASE.parent)!r})",
                    "import g_resource,g_closed_guard",
                    "def sample(*_):",
                    " time.sleep(.04)",
                    f" return dict(root_free={0 if failure=='floor' else 40*1024**3},task_free=300*1024**3,memory_available=20*1024**3,owned=0,measurement_retries=[])",
                    "g_resource.resource_sample=sample",
                    f"g_closed_guard.discover=lambda _:dict(allocation={str(allocation)!r},mode={mode!r},root={str(allocation/'frozen')!r})",
                    f"sys.argv=[{str(PHASE)!r},'entry-refusal-control',sys.executable,'-c',{payload!r}]",
                    f"runpy.run_path({str(PHASE)!r},run_name='__main__')",
                ])
                child = subprocess.run([sys.executable, "-c", harness], capture_output=True, text=True, timeout=5,
                    env=dict(os.environ, S30_SOURCE=str(PHASE.parents[3]), S30_TASK=str(root)), check=False)
                self.assertNotEqual(child.returncode, 0)
                self.assertFalse(marker.exists())
                budget = json.loads((allocation / "g-budget.json").read_text())
                self.assertGreaterEqual(budget["used_ns"]-prior, 40_000_000,
                    "failed common entry lost its real sampler/admission interval")
                self.assertEqual(budget["used_ns"]-prior, budget["common_used_ns"][mode]-common)
                self.assertEqual(budget["cell_used_ns"], {"g1-n100000": 657935962544})
                self.assertEqual(budget["exploratory_seed100k"], token)
                self.assertIsNone(budget["inflight"])
