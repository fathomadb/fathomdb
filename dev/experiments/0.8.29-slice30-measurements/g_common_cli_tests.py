"""Private metadata workers require live common admission; no G workload is invoked."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

RUNNER = Path(__file__).with_name("g_runner.py").resolve()


def subject(root):
    frozen = root / "allocation/frozen"
    frozen.mkdir(parents=True)
    (frozen / "manifest.json").write_text(json.dumps({"plan": [], "allocation": str(frozen.parent),
        "preparation": "closed-copy-v1", "stage": "closed-copy-calibration"}))
    return frozen


class GCommonCliTests(unittest.TestCase):
    def test_bare_common_cli_cannot_publish_uncharged_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            frozen = subject(root)
            job = root / "job.json"
            job.write_text(json.dumps({"action": "report", "options": {"root": str(frozen)}}))
            config = root / "config.json"
            config.write_text('{"id":"unadmitted-common"}')
            output = root / "output"
            output.mkdir()
            environment = dict(os.environ)
            environment.pop("S30_G_COMMON_ADMISSION", None)
            child = subprocess.run([sys.executable, str(RUNNER), "_common", str(job), str(config), str(output)],
                capture_output=True, text=True, timeout=5, env=environment, check=False)
            self.assertFalse(list(frozen.glob("summary*.json")), "bare common worker bypassed budget/guardian and published output")
            self.assertNotEqual(child.returncode, 0)
            self.assertFalse((output / "result.json").exists())

    def test_actual_owned_public_report_transport_still_funds_the_common_worker(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            frozen = subject(root)
            for name in ("logs", "receipts"):
                (root / name).mkdir()
            controller = "\n".join([
                "import pathlib,sys",
                f"sys.path.insert(0,{str(RUNNER.parent)!r})",
                "from g_closed_entry import report_entry",
                f"value=report_entry(pathlib.Path({str(frozen)!r}))",
                "assert value['complete'] is False",
            ])
            phase = RUNNER.with_name("g_phase.py")
            harness = "\n".join([
                "import runpy,sys",
                f"sys.path.insert(0,{str(RUNNER.parent)!r})",
                "import g_resource",
                "# Synthetic lifecycle observations only; actual ownership/cgroup controls remain active.",
                "g_resource.resource_sample=lambda *_:dict(root_free=40*1024**3,task_free=300*1024**3,memory_available=20*1024**3,owned=0,measurement_retries=[])",
                f"sys.argv=[{str(phase)!r},'common-cli-control',sys.executable,'-c',{controller!r}]",
                f"runpy.run_path({str(phase)!r},run_name='__main__')",
            ])
            child = subprocess.run([sys.executable, "-c", harness], capture_output=True, text=True, timeout=10,
                env=dict(os.environ, S30_TASK=str(root), S30_SOURCE=str(RUNNER.parents[3])), check=False)
            logs = (root / "logs/common-cli-control.log").read_text(errors="replace")
            self.assertEqual(child.returncode, 0, child.stdout+child.stderr+logs)
            self.assertEqual(len(list(frozen.glob("summary*.json"))), 1)
            budget = json.loads((frozen.parent / "g-budget.json").read_text())
            self.assertGreater(budget["common_used_ns"]["closed-copy-calibration"], 0)
            self.assertIsNone(budget["inflight"])
            receipt = json.loads((root / "receipts/common-cli-control.json").read_text())
            self.assertFalse(receipt["cleanup_uncertain"])
            self.assertFalse(receipt["boundary"]["populated_after"])
            self.assertTrue(receipt["boundary"]["removed_exact_empty_boundary"])
