#!/usr/bin/env python3
"""Keep the actual lease while cleanup is uncertain; failed cleanup cannot qualify."""

import fcntl
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

import test_cuda_cpu_nonuse as oracle

TOOL_PATH = Path(
    os.environ.get(
        "AC082D_TOOL_UNDER_TEST", oracle.ROOT / "scripts/release/cuda-cpu-nonuse.py"
    )
)
SPEC = importlib.util.spec_from_file_location("nonuse_cleanup", TOOL_PATH)
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)


class QuiescenceTests(unittest.TestCase):
    def test_failed_or_uncertain_cleanup_cannot_qualify(self) -> None:
        for extra in (
            {"cleanup_error": "cleanup failed", "cleanup_uncertain": False},
            {"cleanup_uncertain": True},
        ):
            with self.subTest(extra=extra), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                oracle.MeasurementTests().fixture(root)
                path = root / "observations.json"
                record = json.loads(path.read_text()) | extra
                path.write_text(json.dumps(record))
                with self.assertRaises(ValueError):
                    TOOL.validate_capture(root, "cpu", oracle.UUID)

    def test_cleanup_refusal_keeps_lease_until_consumer_quiesces(self) -> None:
        sys.path.insert(
            0, str(oracle.ROOT / "dev/experiments/0.8.29-slice30-measurements")
        )
        from g_boundary_tests import PinnedRescue, actor_facts

        program = r"""
import fcntl,importlib.util,json,os,pathlib,signal,sys,time
from unittest.mock import patch
spec=importlib.util.spec_from_file_location("capture",sys.argv[1])
tool=importlib.util.module_from_spec(spec); spec.loader.exec_module(tool)
fd=int(sys.argv[3]); os.dup2(fd,9)
if fd!=9: os.close(fd)
fcntl.flock(9,fcntl.LOCK_EX)
root=pathlib.Path(sys.argv[2]); owned={}; original=tool.start_trace
if hasattr(tool,"GROUP"):
    def refuse(_process): raise RuntimeError("injected bounded cleanup refusal")
    tool.GROUP.stop_owned_group=refuse
def start(*args):
    owned["tracer"]=original(*args); return owned["tracer"]
def observe(_uuid):
    if "tracer" in owned and not owned.get("killed") and (root/"ready.json").exists():
        owned["killed"]=True; os.kill(owned["tracer"].pid,signal.SIGKILL); time.sleep(.05)
    return []
consumer="import json,os,pathlib,time; p=pathlib.Path(os.environ['FATHOMDB_CAPTURE_DIR']); (p/'ready.json').write_text(json.dumps({'pid':os.getpid()})); time.sleep(30)"
with patch.object(tool,"start_trace",side_effect=start), patch.object(tool,"query_compute",side_effect=observe):
    tool.capture("python","cuda:0",[sys.executable,"-c",consumer],root,os.environ.copy(),"unused",8)
"""
        rescue = PinnedRescue()
        with tempfile.TemporaryDirectory() as temp:
            task = Path(temp)
            root, lease = task / "capture", task / "lease"
            with lease.open("w") as descriptor, (task / "worker.log").open("w") as log:
                worker = subprocess.Popen(
                    [
                        sys.executable,
                        "-c",
                        program,
                        str(TOOL_PATH),
                        str(root),
                        str(descriptor.fileno()),
                    ],
                    pass_fds=(descriptor.fileno(),),
                    stdout=log,
                    stderr=log,
                    start_new_session=True,
                )
            try:
                until = time.monotonic() + 8
                while (
                    not (root / "observations.json").exists()
                    and time.monotonic() < until
                ):
                    time.sleep(0.02)
                ready = json.loads((root / "ready.json").read_text())
                handle = rescue.pin(actor_facts(ready["pid"]))
                record = json.loads((root / "observations.json").read_text())
                self.assertTrue(
                    record.get("cleanup_uncertain"),
                    "capture released custody without proving cleanup",
                )
                self.assertIsNone(worker.poll())
                with lease.open("r") as probe:
                    with self.assertRaises(BlockingIOError):
                        fcntl.flock(probe, fcntl.LOCK_EX | fcntl.LOCK_NB)
                rescue.send(handle, 9)
                self.assertNotEqual(worker.wait(timeout=5), 0)
                with lease.open("r") as probe:
                    fcntl.flock(probe, fcntl.LOCK_EX | fcntl.LOCK_NB)
                self.assertFalse(
                    json.loads((root / "observations.json").read_text())[
                        "cleanup_uncertain"
                    ]
                )
            finally:
                rescue.close()
                worker.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
