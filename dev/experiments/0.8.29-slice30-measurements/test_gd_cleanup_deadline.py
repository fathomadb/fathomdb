"""Actual owned-group cleanup cannot renew an absolute cleanup endpoint."""

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

from gd_resources import _cleanup_owned_group, _owned_subreaper


def cleanup_case(exited_leader):
    root = Path(tempfile.mkdtemp(prefix="s30-cleanup-deadline-"))
    ready = root / "ready.json"
    body = ("import json,os,pathlib,signal,time; "
            "signal.signal(signal.SIGTERM,signal.SIG_IGN); "
            f"pathlib.Path({str(ready)!r}).write_text(json.dumps({{'pid':os.getpid(),'pgid':os.getpgrp()}})); "
            "print('owned-cleanup-ready',flush=True); time.sleep(60)")
    if exited_leader:
        body = ("import pathlib,subprocess,sys,time\n"
                f"subprocess.Popen([sys.executable,'-c',{body!r}])\n"
                "deadline=time.monotonic()+5\n"
                f"while not pathlib.Path({str(ready)!r}).exists():\n"
                " if time.monotonic()>deadline: raise RuntimeError('owned fixture not ready')\n"
                " time.sleep(0.002)\n")
    with _owned_subreaper(), (root / "stdout.log").open("wb") as stdout, (root / "stderr.log").open("wb") as stderr:
        child = subprocess.Popen([sys.executable,"-c",body], stdout=stdout, stderr=stderr, start_new_session=True)
        try:
            start = time.monotonic()
            while not ready.exists() or (exited_leader and child.poll() is None):
                if time.monotonic()-start > 5:
                    raise AssertionError("cleanup fixture startup failed NOT_RED")
                time.sleep(0.002)
            identity = json.loads(ready.read_text())
            assert identity["pgid"] == child.pid
            assert os.getpgid(identity["pid"]) == child.pid
            start = time.monotonic()
            report = _cleanup_owned_group(child, deadline=start+0.3)
            elapsed = time.monotonic()-start
            gone = False
            try:
                os.kill(identity["pid"], 0)
            except ProcessLookupError:
                gone = True
            print(json.dumps({"root":str(root),"exited_leader":exited_leader,"elapsed":elapsed,
                              "leader_exit":child.returncode,"report":report,"owned_pid_gone":gone}), flush=True)
            return (elapsed < 0.8, report["complete"], gone,
                    child.returncode == (0 if exited_leader else -signal.SIGKILL))
        finally:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            end = time.monotonic()+2
            while time.monotonic() < end:
                child.poll()
                try:
                    pid, _ = os.waitpid(-child.pid, os.WNOHANG)
                except ChildProcessError:
                    break
                if not pid:
                    time.sleep(0.002)


class CleanupDeadlineTests(unittest.TestCase):
    def test_ignoring_owned_leader_does_not_get_new_fifteen_seconds(self):
        self.assertEqual(cleanup_case(False), (True, True, True, True))

    def test_exited_leader_does_not_renew_live_descendant_cleanup(self):
        self.assertEqual(cleanup_case(True), (True, True, True, True))


if __name__ == "__main__":
    unittest.main()
