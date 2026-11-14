"""Public internal-cell CLI cannot dispatch a program without active owned admission."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

from g_campaign import make_g_plan
from test_g_closed_copy import tiny_source
from test_g_closed_modes import references
from test_g_closed_plan import SPEC

RUNNER = Path(__file__).with_name("g_runner.py").resolve()


class GClosedCellCliTests(unittest.TestCase):
    def test_bare_cli_never_copies_or_dispatches_with_or_without_long_optin(self):
        for long in (False, True):
            with self.subTest(long_optin=long), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                calls = root / "forbidden"
                program = [sys.executable, "-c", "\n".join([
                    "import json,pathlib,sys",
                    f"pathlib.Path({str(calls)!r}).touch()",
                    "o=pathlib.Path(sys.argv[2]);o.mkdir();(o/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True)))",
                ])]
                spec = dict(SPEC, preparation="closed-copy-v1")
                manifest = {"schema": "slice30-g-campaign/v1", "preparation": "closed-copy-v1",
                    "stage": "closed-copy-read-r1", "spec": spec, "plan": make_g_plan(spec),
                    "closed_subject": tiny_source(root), "commands": {"ordinary": program, "diagnostic": program},
                    "allocation": str(root), "corpus": "unused-control", "queries": "unused-control"}
                manifest_path = root / "prepared.json"
                manifest_path.write_text(json.dumps(manifest))
                (root / "results").mkdir()
                (root / "results/campaign-state.json").write_text(json.dumps(list(references().values())))
                cell = next(row for row in manifest["plan"] if row["id"] == "g-r-n100000-r1")
                config = root / "arbitrary-cell.json"
                config.write_text(json.dumps(cell))
                output = root / "unadmitted"
                environment = dict(os.environ)
                environment.pop("AGENT_LONG", None)
                environment.pop("S30_G_CELL_ADMISSION", None)
                if long:
                    environment["AGENT_LONG"] = "1"
                child = subprocess.run([sys.executable, str(RUNNER), "_cell", str(manifest_path),
                    str(time.monotonic_ns()+5*10**9), str(config), str(output)],
                    env=environment, capture_output=True, text=True, timeout=10, check=False)
                self.assertFalse(calls.exists(), "public _cell bypassed controller/stage admission")
                self.assertFalse((output / "subject").exists(), "unadmitted CLI copied the released subject")
                self.assertNotEqual(child.returncode, 0)

    def test_real_owned_controller_transport_remains_usable_and_protocol_only(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            allocation = root / "allocation"
            frozen = allocation / "frozen"
            frozen.mkdir(parents=True)
            for name in ("logs", "receipts"):
                (root / name).mkdir()
            calls = root / "admitted-program"
            program = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sys",
                f"pathlib.Path({str(calls)!r}).touch()",
                "o=pathlib.Path(sys.argv[2]);o.mkdir();(o/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True)))",
            ])]
            spec = dict(SPEC, preparation="closed-copy-v1")
            prediction = {"allowed": True, "total_ns": 1, "new_bytes": 1}
            manifest = {"schema": "slice30-g-campaign/v1", "preparation": "closed-copy-v1",
                "stage": "closed-copy-read-r1", "spec": spec, "plan": make_g_plan(spec),
                "closed_subject": tiny_source(root), "commands": {"ordinary": program, "diagnostic": program},
                "root": str(frozen), "allocation": str(allocation), "corpus": "unused-control",
                "queries": "unused-control", "_closed_predictions": {"g-r-n100000-r1": prediction}}
            mf = frozen / "prepared.json"
            mf.write_text(json.dumps(manifest))
            controller = "\n".join([
                "import json,pathlib,sys",
                f"sys.path.insert(0,{str(RUNNER.parent)!r})",
                "from g_campaign import run_g_plan",
                "from test_g_closed_modes import references",
                "from test_g_closed_plan import SPEC",
                f"def command(cell,deadline):return [sys.executable,{str(RUNNER)!r},'_cell',{str(mf)!r},str(deadline)]",
                f"def prepare(*_):return dict(prepared_manifest={str(mf)!r},prospective_prediction={prediction!r})",
                f"run_g_plan(dict(SPEC,preparation='closed-copy-v1',closed_mode='closed-copy-read-r1'),pathlib.Path({str(frozen/'results')!r}),command,prepare=prepare,allocation_root=pathlib.Path({str(allocation)!r}),reused=references())",
            ])
            phase = RUNNER.with_name("g_phase.py")
            harness = "\n".join([
                "import pathlib,runpy,sys",
                f"sys.path.insert(0,{str(RUNNER.parent)!r})",
                "import g_resource,g_closed_guard",
                "# Synthetic source-test resources; real cgroup/ownership remain active.",
                "g_resource.resource_sample=lambda *_:dict(root_free=40*1024**3,task_free=300*1024**3,memory_available=20*1024**3,owned=0,measurement_retries=[])",
                f"g_closed_guard.discover=lambda _:dict(allocation={str(allocation)!r},mode='closed-copy-read-r1',root={str(frozen)!r})",
                f"sys.argv=[{str(phase)!r},'admitted-cell-control',sys.executable,'-c',{controller!r}]",
                f"runpy.run_path({str(phase)!r},run_name='__main__')",
            ])
            child = subprocess.run([sys.executable, "-c", harness], capture_output=True, text=True, timeout=15,
                env=dict(os.environ, S30_SOURCE=str(RUNNER.parents[3]), S30_TASK=str(root), AGENT_LONG="1"),
                check=False)
            logs = "\n".join(path.read_text(errors="replace") for path in root.rglob("child.log"))
            self.assertEqual(child.returncode, 0, child.stdout+child.stderr+logs)
            self.assertTrue(calls.exists(), "valid controller admission transport refused\n"+logs)
            row = next(row for row in json.loads((frozen/"results/g-outcomes.json").read_text())
                       if row["id"] == "g-r-n100000-r1")
            self.assertEqual(row["status"], "failed", "protocol-only command became Default/GPU qualification")
            result = json.loads(Path(row["attempt"], "result.json").read_text())
            self.assertIn("runner_only_control", result["qualification_issues"])
            self.assertFalse(json.loads(next(root.rglob("admitted-cell-control.json")).read_text())["cleanup_uncertain"])
