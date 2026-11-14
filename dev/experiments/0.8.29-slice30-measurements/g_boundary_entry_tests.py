"""Actual unknown-cleanup state must block later guards and normal G dispatch."""

from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from g_campaign import make_g_plan
from g_runner import run_frozen
from g_boundary_tests import PHASE, PinnedRescue, actor_facts


@contextmanager
def unknown_boundary(root):
    """Keep original private controls pinned while the guardian loses population observations."""
    for name in ("logs", "receipts", "allocation"):
        (root / name).mkdir()
    ready, release, completed = (root / name for name in ("ready", "release", "completed"))
    payload = "\n".join([
        "import json,pathlib,time",
        f"pathlib.Path({str(ready)!r}).touch()",
        "until=time.monotonic()+3",
        f"while not pathlib.Path({str(release)!r}).exists() and time.monotonic()<until: time.sleep(0.005)",
        f"assert pathlib.Path({str(release)!r}).exists()",
        f"pathlib.Path({str(root / 'allocation/g-budget.json')!r}).write_text(json.dumps(dict(schema='slice30-gd-budget/v1',used_ns=17,inflight=None,cell_used_ns={{}})))",
        f"pathlib.Path({str(completed)!r}).touch()",
    ])
    harness = "\n".join([
        "import errno,os,pathlib,runpy,sys",
        f"sys.path.insert(0,{str(PHASE.parent)!r})",
        "import g_resource",
        "g_resource.resource_sample=lambda *_:dict(root_free=40*1024**3,task_free=300*1024**3,memory_available=20*1024**3,owned=0,measurement_retries=[])",
        "actual_read=os.read",
        "def read(fd,count):",
        f"    if pathlib.Path({str(completed)!r}).exists() and os.readlink(f'/proc/self/fd/{{fd}}').endswith('/cgroup.events'): raise OSError(errno.EIO,'controlled unknown population after payload completed')",
        "    return actual_read(fd,count)",
        "os.read=read",
        f"sys.argv=[{str(PHASE)!r},'unknown-control',sys.executable,'-c',{payload!r}]",
        f"runpy.run_path({str(PHASE)!r},run_name='__main__')",
    ])
    rescue = PinnedRescue()
    directory = kill = events = None
    guardian = None
    state = None
    try:
        with (root / "guardian.log").open("w") as log:
            guardian = subprocess.Popen([sys.executable, "-c", harness], stdout=log, stderr=subprocess.STDOUT,
                                        env=dict(os.environ, S30_SOURCE=str(PHASE.parents[3]), S30_TASK=str(root)))
            rescue.pin(actor_facts(guardian.pid))
            deadline = time.monotonic() + 4
            while not ready.exists():
                if guardian.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError("unknown-state coverage unavailable before payload readiness: " + (root / "guardian.log").read_text())
                time.sleep(0.005)
            state = json.loads((root / "g-containment-state.json").read_text())
            directory = os.open(state["path"], os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            identity = os.fstat(directory)
            assert (identity.st_dev, identity.st_ino) == (state["identity"]["device"], state["identity"]["inode"])
            kill = os.open("cgroup.kill", os.O_WRONLY | os.O_CLOEXEC, dir_fd=directory)
            events = os.open("cgroup.events", os.O_RDONLY | os.O_CLOEXEC, dir_fd=directory)
            release.touch()
            guardian.wait(timeout=8)
        report = json.loads((root / "receipts/unknown-control.json").read_text())
        assert guardian.returncode != 0 and report["cleanup_uncertain"]
        state_bytes = (root / "g-containment-state.json").read_bytes()
        assert json.loads(state_bytes)["status"] == "unknown"
        assert json.loads((root / "allocation/g-budget.json").read_text())["inflight"] is None
        yield state_bytes
    finally:
        release.touch()
        rescue.close()
        if guardian is not None:
            guardian.wait(timeout=2)
        if kill is not None:
            os.write(kill, b"1")
            until = time.monotonic() + 2
            while True:
                os.lseek(events, 0, os.SEEK_SET)
                values = dict(line.split() for line in os.read(events, 4096).decode().splitlines())
                if values["populated"] == "0":
                    break
                if time.monotonic() >= until:
                    raise RuntimeError("test's held private boundary rescue remains populated")
                time.sleep(0.005)
            actual = os.stat(state["path"], follow_symlinks=False)
            held = os.fstat(directory)
            assert (actual.st_dev, actual.st_ino) == (held.st_dev, held.st_ino)
            os.rmdir(state["path"])
        for descriptor in (events, kill, directory):
            if descriptor is not None:
                os.close(descriptor)


class GUnknownBoundaryEntryTests(unittest.TestCase):
    def test_unknown_cleanup_cannot_be_overwritten_by_following_guard(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with unknown_boundary(root) as original:
                marker = root / "unsafe-following-phase"
                command = [sys.executable, str(PHASE), "following-control", sys.executable, "-c",
                           f"import pathlib; pathlib.Path({str(marker)!r}).touch()"]
                child = subprocess.run(command, capture_output=True, text=True, timeout=10, check=False,
                                       env=dict(os.environ, S30_SOURCE=str(PHASE.parents[3]), S30_TASK=str(root)))
                self.assertFalse(marker.exists(), "following guardian dispatched despite retained unknown cleanup")
                self.assertNotEqual(child.returncode, 0)
                self.assertEqual((root / "g-containment-state.json").read_bytes(), original)
                self.assertEqual(json.loads((root / "allocation/g-budget.json").read_text())["used_ns"], 17)

    def test_unknown_cleanup_blocks_normal_entry_even_with_cleared_shared_inflight(self):
        from g_freeze import git

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with unknown_boundary(root) as original:
                frozen = root / "allocation/frozen"
                frozen.mkdir()
                calls = root / "unsafe-normal-dispatch"
                command = root / "harmless-command"
                command.write_text(f"#!{sys.executable}\nimport json,pathlib,sys\n"
                                   f"pathlib.Path({str(calls)!r}).touch()\n"
                                   "out=pathlib.Path(sys.argv[2]); out.mkdir()\n"
                                   "(out/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True)))\n")
                command.chmod(0o555)
                spec = json.loads(PHASE.with_name("g-cells.json").read_text())
                manifest = dict(schema="slice30-g-campaign/v1", source_sha=git("rev-parse", "HEAD"),
                                allocation=str(root / "allocation"), plan=make_g_plan(spec), spec=spec,
                                inputs={name: {"path": name} for name in ("corpus", "queries")},
                                binaries={name: {"path": str(command)} for name in ("ordinary", "diagnostic")},
                                gpu={"uuid": "GPU-protocol-control"}, device_policy="cuda:0")
                (frozen / "manifest.json").write_text(json.dumps(manifest))
                refusal = None
                # Synthetic frozen/GPU prerequisites qualify dispatch policy only, never model or GPU work.
                gpu = dict(uuid="GPU-protocol-control", applications=[], gpu_memory_mib=0,
                           utilization_percent=0, pid_memory_mib=0)
                with patch.dict(os.environ, S30_TASK=str(root)), \
                        patch("g_freeze.validate_frozen", return_value=manifest), \
                        patch("g_gpu.inspect_gpu", return_value=gpu):
                    try:
                        run_frozen(frozen)
                    except RuntimeError as error:
                        refusal = str(error)
                self.assertFalse(calls.exists(), "normal G entry dispatched after unknown cleanup and cleared inflight")
                self.assertIn("containment", refusal or "")
                self.assertEqual((root / "g-containment-state.json").read_bytes(), original)
                self.assertEqual(json.loads((root / "allocation/g-budget.json").read_text())["used_ns"], 17)
