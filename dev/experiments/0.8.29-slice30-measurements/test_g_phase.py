"""Real harmless descendants qualify the owned native executor's cleanup contract."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest


PHASE = Path(__file__).with_name("g_phase.py").resolve()


def phase_control(root, *, resource_stop):
    """Run actual process groups; only resource observations and polling cadence are controlled."""
    ready, trigger, late = (root / name for name in ("ready.json", "trigger", "late"))
    descendant = "\n".join([
        "import json,os,pathlib,signal,time",
        "signal.signal(signal.SIGTERM,signal.SIG_IGN)",
        f"pathlib.Path({str(ready)!r}).write_text(json.dumps({{'pid':os.getpid(),'pgid':os.getpgrp(),'sid':os.getsid(0)}}))",
        "until=time.monotonic()+2",
        f"while {resource_stop!r} and not pathlib.Path({str(trigger)!r}).exists() and time.monotonic()<until: time.sleep(0.005)",
        "time.sleep(0.15)",
        f"pathlib.Path({str(late)!r}).write_text('survived native phase leader')",
    ])
    leader = "\n".join([
        "import pathlib,subprocess,sys,time",
        f"subprocess.Popen([sys.executable,'-c',{descendant!r}])",
        "until=time.monotonic()+2",
        f"while not pathlib.Path({str(ready)!r}).exists() and time.monotonic()<until: time.sleep(0.005)",
        f"time.sleep(2 if {resource_stop!r} else 0)",
    ])
    harness = "\n".join([
        "import pathlib,runpy,subprocess,sys",
        f"sys.path.insert(0,{str(PHASE.parent)!r})",
        "import g_resource",
        "actual_sample=g_resource.resource_sample; actual_wait=subprocess.Popen.wait",
        "def sample(source,task):",
        "    values=actual_sample(source,task)",
        f"    if {resource_stop!r} and pathlib.Path({str(ready)!r}).exists():",
        f"        pathlib.Path({str(trigger)!r}).write_text('controlled resource stop')",
        "        values['memory_available']=0",
        "    return values",
        "def wait(child,timeout=None):",
        "    return actual_wait(child,timeout=0.05 if timeout==30 else timeout)",
        "g_resource.resource_sample=sample; subprocess.Popen.wait=wait",
        f"sys.argv=[{str(PHASE)!r},'controlled-phase',sys.executable,'-c',{leader!r}]",
        f"runpy.run_path({str(PHASE)!r},run_name='__main__')",
    ])
    for name in ("logs", "receipts"):
        (root / name).mkdir()
    environment = dict(os.environ, S30_TASK=str(root))
    child = subprocess.run([sys.executable, "-c", harness], env=environment,
                           text=True, capture_output=True, timeout=15, check=False)
    time.sleep(0.35)
    report = json.loads((root / "receipts/controlled-phase.json").read_text())
    facts = json.loads(ready.read_text())
    print(json.dumps({"phase": report, "descendant": facts, "late_write": late.exists()}), flush=True)
    return child, report, facts, late


class GPhaseResourceStopTests(unittest.TestCase):
    def test_resource_stop_cleans_term_resistant_descendant_after_leader_exits(self):
        with tempfile.TemporaryDirectory() as temporary:
            child, report, facts, late = phase_control(Path(temporary), resource_stop=True)
            self.assertIsNotNone(report["resource_stop"], child.stdout + child.stderr)
            self.assertNotEqual(child.returncode, 0)
            self.assertEqual((facts["pgid"], facts["sid"]), (report["pid"], report["pid"]))
            self.assertNotEqual(facts["pid"], report["pid"])
            self.assertFalse(late.exists(), "TERM-resistant actual descendant survived resource-stop leader cleanup")
