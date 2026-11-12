"""Owned descendants cannot outlive their guarded phase's cleanup."""

import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest


class OwnedGroupTests(unittest.TestCase):
    def check_group(self, immediate_exit):
        libc = ctypes.CDLL(None, use_errno=True)
        previous = ctypes.c_int()
        self.assertEqual(libc.prctl(37, ctypes.byref(previous), 0, 0, 0), 0)
        self.assertEqual(libc.prctl(36, 1, 0, 0, 0), 0)
        descendant_pid = None
        try:
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                ready, payload = root / "descendant.json", root / "payload"
                descendant = (
                    "import json,os,pathlib,signal,time; "
                    "signal.signal(signal.SIGTERM,signal.SIG_IGN); "
                    f"pathlib.Path({str(ready)!r}).write_text(json.dumps({{'pid':os.getpid(),'pgid':os.getpgrp()}})); "
                    "time.sleep(60)"
                )
                leader = (
                    "import pathlib,subprocess,sys,time\n"
                    f"subprocess.Popen([sys.executable,'-c',{descendant!r}])\n"
                    "deadline=time.monotonic()+5\n"
                    f"while not pathlib.Path({str(ready)!r}).exists():\n"
                    " if time.monotonic()>deadline: raise RuntimeError('fixture child not ready')\n"
                    " time.sleep(0.005)\n"
                    "print('owned-descendant-ready',flush=True)\n"
                    f"pathlib.Path({str(payload)!r}).write_bytes(b'x'*131072)\n"
                    + ("" if immediate_exit else "time.sleep(60)\n")
                )
                tool = Path(__file__).resolve().parent
                guard = (
                    "import json,sys\n"
                    f"sys.path.insert(0,{str(tool)!r})\n"
                    "from gd_resources import run_guarded\n"
                    f"result=run_guarded({[sys.executable,'-c',leader]!r},{str(root)!r},"
                    f"roots={[str(root)]!r},baseline=0,max_owned=65536,interval={30 if immediate_exit else 0.02})\n"
                    "print(json.dumps(result),flush=True)\n"
                )
                child = subprocess.Popen([sys.executable, "-c", guard], stdout=subprocess.PIPE,
                                         stderr=subprocess.PIPE, text=True)
                try:
                    deadline = time.monotonic() + 5
                    while not ready.exists() and time.monotonic() < deadline:
                        time.sleep(0.01)
                    info = json.loads(ready.read_text())
                    descendant_pid = info["pid"]
                    stdout, stderr = child.communicate(timeout=35)
                finally:
                    if child.poll() is None:
                        child.kill()
                    child.wait()
                self.assertEqual(child.returncode, 0, stderr)
                result = json.loads(stdout)
                print(json.dumps(result), flush=True)
                self.assertEqual(info["pgid"], result["pid"])
                self.assertIn("owned-descendant-ready", (root / "logs/phase.log").read_text())
                self.assertEqual(payload.stat().st_size, 131072)
                self.assertEqual(result["exit"], 124)
                try:
                    os.kill(descendant_pid, 0)
                    present = True
                except ProcessLookupError:
                    present = False
                self.assertFalse(present, "owned descendant survived guard disposition")
        finally:
            # The outer fixture adopts leaked RED descendants solely to clean them.
            if descendant_pid is not None:
                try:
                    os.kill(descendant_pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    try:
                        reaped, _ = os.waitpid(descendant_pid, os.WNOHANG)
                    except ChildProcessError:
                        break
                    if reaped:
                        break
                    time.sleep(0.01)
            self.assertEqual(libc.prctl(36, previous.value, 0, 0, 0), 0)

    def test_term_exited_leader_does_not_leave_ignoring_descendant(self):
        self.check_group(False)

    def test_already_exited_leader_does_not_bypass_group_cleanup(self):
        self.check_group(True)


if __name__ == "__main__":
    unittest.main()
