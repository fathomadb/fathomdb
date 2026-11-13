"""Real harmless-descendant regression for pre-registration cleanup; no GDB/product."""

import ctypes
import importlib.util
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
from types import SimpleNamespace
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("slice100_cleanup_regression", Path(__file__).with_name("operator.py"))
control = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(control)

LEAF = 'import os,time; print(os.getpid(),flush=True); time.sleep(60)'
ROOT = 'import subprocess,sys,time; p=subprocess.Popen([sys.executable,"-u","-c",sys.argv[1]],stdout=subprocess.PIPE); print(p.stdout.readline().decode().strip(),flush=True); time.sleep(60)'


def supervisor(mode: str, directory: Path) -> None:
    """Exercise actual run_case cleanup with harmless transport and an independent safeguard."""
    # Match the real operator context; the test owns this isolated supervisor process.
    if ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) != 0:
        raise RuntimeError('test subreaper unavailable')
    launched = {}

    def harmless_transport(args, case, nonce, actors):
        root = subprocess.Popen([sys.executable, '-u', '-c', ROOT, LEAF], stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE)
        launched['root'] = root
        actors.processes.append(root)
        if not select.select([root.stdout], [], [], 3)[0]:
            raise RuntimeError('harmless descendant readiness deadline')
        leaf = int(root.stdout.readline())
        launched['leaf'] = leaf
        if mode == 'pidfd_failure':
            with patch.object(control.os, 'pidfd_open', side_effect=OSError('injected pidfd registration failure')):
                actors.bind_inferior(leaf, root.pid, Path(sys.executable).resolve())
        raise control.Unqualified('injected EOF/resource/decode failure before MI registration')

    args = SimpleNamespace(technical_go=True, mode='healthy', task_root=directory,
                           source_root=directory, case_root=directory/'case')
    try:
        with patch.object(control, 'source_subject', return_value={'harmless_control': True}), \
             patch.object(control, 'resource_sample', return_value={}), \
             patch.object(control, 'check_resources'), \
             patch.object(control, 'Debugger', side_effect=harmless_transport):
            result = control.run_case(args)
        leaf = launched['leaf']
        absent = not Path(f'/proc/{leaf}').exists()
        evidence = dict(mode=mode, descendant_pid=leaf, descendant_absent=absent,
                        cleanup_errors=result.get('cleanup_errors', []), status=result['status'])
        print(json.dumps(evidence), flush=True)
        assert absent, 'registered launch leaked a live/unreaped harmless descendant after cleanup returned'
        assert not evidence['cleanup_errors'], 'cleanup did not finish within its bounds'
        assert result['status'] == 'UNQUALIFIED', 'failure cleanup must never qualify a case'
    finally:
        # Independent outer safeguard: it does not call the operator cleanup being tested.
        root = launched.get('root')
        if root:
            if root.poll() is None:
                root.kill()
            root.wait(timeout=5)
        leaf = launched.get('leaf')
        if leaf and Path(f'/proc/{leaf}').exists():
            stat = Path(f'/proc/{leaf}/stat').read_text().rsplit(')', 1)[1].split()
            assert int(stat[1]) == os.getpid(), 'safeguard refuses an actor outside its direct adopted children'
            # A direct unreaped child cannot have this PID reused by another process.
            os.kill(leaf, signal.SIGKILL)
            until = time.monotonic() + 5
            while True:
                try:
                    reaped, _ = os.waitpid(leaf, os.WNOHANG)
                except ChildProcessError:
                    break
                if reaped:
                    break
                if time.monotonic() >= until:
                    raise RuntimeError('independent safeguard reap deadline')
                time.sleep(0.01)
        if root:
            root.stdout.close()
            root.stderr.close()


@unittest.skipUnless(sys.platform == 'linux', 'Linux subreaper ownership regression')
class DescendantCleanupRegression(unittest.TestCase):
    """An isolated process group bounds even a broken operator/supervisor cleanup path."""

    def test_unregistered_and_pidfd_failure_descendants_are_reaped(self):
        for mode in ['before_registration', 'pidfd_failure']:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(prefix='s100-cleanup-control-') as tmp:
                child = subprocess.Popen([sys.executable, '-I', str(Path(__file__).resolve()), '--supervise', mode, tmp],
                                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
                try:
                    try:
                        stdout, stderr = child.communicate(timeout=12)
                    except subprocess.TimeoutExpired:
                        # The known unreaped group leader was created by this exact spawn.
                        os.killpg(child.pid, signal.SIGKILL)
                        stdout, stderr = child.communicate(timeout=5)
                        self.fail('outer safeguard deadline: ' + stderr.decode(errors='replace'))
                    self.assertEqual(child.returncode, 0, stdout.decode() + stderr.decode())
                    self.assertTrue(json.loads(stdout)['descendant_absent'])
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait(timeout=5)
                    child.stdout.close()
                    child.stderr.close()


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == '--supervise':
        supervisor(sys.argv[2], Path(sys.argv[3]))
    else:
        unittest.main()
