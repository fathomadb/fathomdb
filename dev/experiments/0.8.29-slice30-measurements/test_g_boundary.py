"""Real composed sessions and controlled pre-exec faults; never GPU qualification."""

import ctypes
import errno
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import time
import unittest

PHASE = Path(__file__).with_name("g_phase.py").resolve()


def actor_facts(pid):
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()
    return dict(pid=pid, state=fields[0], parent=int(fields[1]), pgid=int(fields[2]),
                sid=int(fields[3]), start=int(fields[19]))


class PinnedRescue:
    """Test-only libc handles pin exact actors; no numeric signalling fallback."""

    def __init__(self):
        self.libc = ctypes.CDLL(None, use_errno=True)
        self.libc.pidfd_open.argtypes = [ctypes.c_int, ctypes.c_uint]
        self.libc.pidfd_open.restype = ctypes.c_int
        self.libc.pidfd_send_signal.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p, ctypes.c_uint]
        self.libc.pidfd_send_signal.restype = ctypes.c_int
        self.handles = []
        descriptor = self.check(self.libc.pidfd_open(os.getpid(), 0))
        try:
            self.check(self.libc.pidfd_send_signal(descriptor, 0, None, 0))
        finally:
            os.close(descriptor)

    def check(self, value):
        if value < 0:
            raise OSError(ctypes.get_errno(), os.strerror(ctypes.get_errno()))
        return value

    def pin(self, facts):
        self.check_identity(facts)
        descriptor = self.check(self.libc.pidfd_open(facts["pid"], 0))
        try:
            self.check_identity(facts)
        except BaseException:
            os.close(descriptor)
            raise
        self.handles.append(descriptor)
        return descriptor

    def check_identity(self, facts):
        current = actor_facts(facts["pid"])
        if any(current[key] != facts[key] for key in ("start", "parent", "pgid", "sid")):
            raise RuntimeError("test actor identity changed before pinning")

    def send(self, descriptor, selected):
        self.check(self.libc.pidfd_send_signal(descriptor, selected, None, 0))

    def close(self):
        for descriptor in reversed(self.handles):
            try:
                self.send(descriptor, signal.SIGKILL)
            except OSError as error:
                if error.errno != errno.ESRCH:
                    raise
            poller = select.poll()
            poller.register(descriptor, select.POLLIN)
            if not poller.poll(2000):
                raise RuntimeError("test-only pinned rescue did not reach terminal state")
            os.close(descriptor)
        self.handles.clear()


def recorded_actor(path):
    return "\n".join([
        "import json,os,pathlib,signal,subprocess,sys,time",
        "fields=pathlib.Path('/proc/self/stat').read_text().rsplit(') ',1)[1].split()",
        f"pathlib.Path({str(path)!r}).write_text(json.dumps(dict(pid=os.getpid(),state=fields[0],parent=int(fields[1]),pgid=int(fields[2]),sid=int(fields[3]),start=int(fields[19]))))",
    ])


def run_composed(test, root, mode):
    rescue = PinnedRescue()
    records = {name: root / f"{name}.json" for name in ("runner", "inner", "native")}
    pinned, released, late = (root / name for name in ("pinned", "guardian-returned", "forbidden"))
    native = recorded_actor(records["native"]) + "\n" + "\n".join([
        "signal.signal(signal.SIGTERM,signal.SIG_IGN)",
        "until=time.monotonic()+20",
        f"while not pathlib.Path({str(released)!r}).exists() and time.monotonic()<until: time.sleep(0.005)",
        f"if pathlib.Path({str(released)!r}).exists(): pathlib.Path({str(late)!r}).write_text('survived guardian completion')",
        "time.sleep(2)",
        "out=pathlib.Path(sys.argv[2]); out.mkdir()",
        "(out/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True)))",
    ])
    inner = recorded_actor(records["inner"]) + "\n" + "\n".join([
        f"sys.path.insert(0,{str(PHASE.parent)!r})",
        "from g_driver import run_program_step",
        f"run_program_step([sys.executable,'-c',{native!r}],dict(family='G-R'),pathlib.Path({str(root / 'native-output')!r}),time.monotonic_ns()+20*10**9)",
    ])
    runner = recorded_actor(records["runner"]) + "\n" + "\n".join([
        "signal.signal(signal.SIGTERM,signal.SIG_IGN)" if mode == "forced" else "pass",
        "signal.signal(signal.SIGUSR1,lambda *_:os._exit(0))" if mode == "normal" else "pass",
        f"sys.path.insert(0,{str(PHASE.parent)!r})",
        "from g_attempt import run_g_attempt",
        f"run_g_attempt(dict(id='composed'),pathlib.Path({str(root / 'attempts')!r}),[sys.executable,'-c',{inner!r}],time.monotonic_ns()+20*10**9)",
    ])
    harness = "\n".join([
        "import pathlib,runpy,sys",
        f"sys.path.insert(0,{str(PHASE.parent)!r})",
        "import g_resource,g_attempt",
        "actual_unreaped=g_attempt.wait_unreaped",
        "def sample(source,task):",
        "    # Synthetic start/stop observations are lifecycle controls, not host readiness.",
        "    values=dict(root_free=40*1024**3,task_free=300*1024**3,memory_available=20*1024**3,owned=0,measurement_retries=[])",
        f"    if {mode != 'normal'!r} and pathlib.Path({str(pinned)!r}).exists(): values['memory_available']=0",
        "    return values",
        "def wait(child,seconds): return actual_unreaped(child,0.05 if seconds==30 else seconds)",
        "g_resource.resource_sample=sample; g_attempt.wait_unreaped=wait",
        f"sys.argv=[{str(PHASE)!r},'composed-control',sys.executable,'-c',{runner!r}]",
        f"runpy.run_path({str(PHASE)!r},run_name='__main__')",
    ])
    for name in ("logs", "receipts"):
        (root / name).mkdir()
    guardian = None
    try:
        with (root / "guardian.log").open("w") as log:
            guardian = subprocess.Popen([sys.executable, "-c", harness], stdout=log, stderr=subprocess.STDOUT,
                                        env=dict(os.environ, S30_SOURCE=str(PHASE.parents[3]), S30_TASK=str(root)))
            rescue.pin(actor_facts(guardian.pid))
            deadline = time.monotonic() + 5
            while not all(path.exists() for path in records.values()):
                if guardian.poll() is not None:
                    raise RuntimeError("composed-chain coverage unavailable: guardian ended before native readiness: " + (root / "guardian.log").read_text())
                if time.monotonic() >= deadline:
                    raise TimeoutError("actual composed chain did not become ready")
                time.sleep(0.005)
            facts = {name: json.loads(path.read_text()) for name, path in records.items()}
            handles = {name: rescue.pin(value) for name, value in facts.items()}
            test.assertEqual(facts["runner"]["parent"], guardian.pid)
            test.assertEqual(facts["inner"]["parent"], facts["runner"]["pid"])
            test.assertEqual(facts["native"]["parent"], facts["inner"]["pid"])
            test.assertNotEqual(facts["inner"]["sid"], facts["runner"]["sid"])
            test.assertEqual(facts["native"]["sid"], facts["inner"]["sid"])
            pinned.touch()
            if mode == "normal":
                rescue.send(handles["runner"], signal.SIGUSR1)
            guardian.wait(timeout=12)
        report = json.loads((root / "receipts/composed-control.json").read_text())
        released.touch()
        time.sleep(0.15)
        live = []
        for name in ("inner", "native"):
            try:
                current = actor_facts(facts[name]["pid"])
                if current["start"] == facts[name]["start"] and current["state"] not in ("Z", "X"):
                    live.append(name)
            except FileNotFoundError:
                pass
        print(json.dumps(dict(mode=mode, report=report, actors=facts, live=live, forbidden=late.exists())), flush=True)
        test.assertFalse(late.exists(), "separately sessioned native survived completed outer guardian")
        test.assertEqual(live, [], "separately sessioned inner/native remained live after guardian")
        test.assertFalse(report["cleanup_uncertain"])
        test.assertNotEqual(guardian.returncode, 0)
        test.assertEqual(report["child_exit"], 0 if mode == "normal" else -9 if mode == "forced" else -15)
    finally:
        rescue.close()
        if guardian is not None:
            guardian.wait(timeout=2)


class GComposedBoundaryTests(unittest.TestCase):
    def test_default_term_stops_real_separate_session_chain(self):
        with tempfile.TemporaryDirectory() as temporary:
            run_composed(self, Path(temporary), "default")

    def test_forced_escalation_stops_real_separate_session_chain(self):
        with tempfile.TemporaryDirectory() as temporary:
            run_composed(self, Path(temporary), "forced")

    def test_successful_runner_cannot_leave_separate_session_work(self):
        with tempfile.TemporaryDirectory() as temporary:
            run_composed(self, Path(temporary), "normal")


class GPreexecBoundaryRefusalTests(unittest.TestCase):
    def run_refusal(self, fault):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ("logs", "receipts"):
                (root / name).mkdir()
            marker = root / "payload-executed"
            payload = f"import pathlib; pathlib.Path({str(marker)!r}).touch()"
            harness = "\n".join([
                "import errno,os,pathlib,runpy,sys",
                f"sys.path.insert(0,{str(PHASE.parent)!r})",
                "import g_resource",
                "g_resource.resource_sample=lambda *_:dict(root_free=40*1024**3,task_free=300*1024**3,memory_available=20*1024**3,owned=0,measurement_retries=[])",
                "actual_mkdir=os.mkdir; actual_write=os.write",
                "def mkdir(path,*args,**kwargs):",
                f"    if {fault == 'create'!r} and str(path).startswith('fathomdb-s30-g-'): raise PermissionError(errno.EACCES,'controlled delegation refusal')",
                "    return actual_mkdir(path,*args,**kwargs)",
                "def write(fd,data):",
                f"    if {fault == 'attach'!r} and os.readlink(f'/proc/self/fd/{{fd}}').endswith('/cgroup.procs'): raise PermissionError(errno.EACCES,'controlled attachment refusal')",
                "    return actual_write(fd,data)",
                "os.mkdir=mkdir; os.write=write",
                f"sys.argv=[{str(PHASE)!r},'refusal-control',sys.executable,'-c',{payload!r}]",
                f"runpy.run_path({str(PHASE)!r},run_name='__main__')",
            ])
            child = subprocess.run([sys.executable, "-c", harness], text=True, capture_output=True, timeout=10,
                                   env=dict(os.environ, S30_SOURCE=str(PHASE.parents[3]), S30_TASK=str(root)), check=False)
            self.assertFalse(marker.exists(), "payload executed despite controlled private-boundary refusal")
            self.assertNotEqual(child.returncode, 0, child.stdout + child.stderr)

    def test_delegation_refusal_prevents_payload_execution(self):
        self.run_refusal("create")

    def test_preexec_attachment_refusal_prevents_payload_execution(self):
        self.run_refusal("attach")
