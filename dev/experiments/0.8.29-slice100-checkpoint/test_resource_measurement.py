"""Controlled real-du child regressions for vanished owned files; no native/product."""

import importlib.util
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("slice100_resource_control", Path(__file__).with_name("operator.py"))
control = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(control)
DU = shutil.which('du')

SHIM = r"""
import os,pathlib,subprocess,sys
counter=pathlib.Path(os.environ['S100_DU_COUNTER'])
attempt=int(counter.read_text())+1 if counter.exists() else 1
counter.write_text(str(attempt))
actual=subprocess.run([os.environ['S100_REAL_DU'],*sys.argv[1:]],capture_output=True)
sys.stdout.buffer.write(actual.stdout)
if actual.returncode:
    sys.stderr.buffer.write(actual.stderr)
    sys.exit(actual.returncode)
mode=os.environ['S100_DU_MODE']
if mode=='once' and attempt>1:
    sys.exit(0)
missing=pathlib.Path(os.environ['S100_DU_MISSING'])
missing.parent.mkdir(parents=True,exist_ok=True)
missing.write_text('owned temporary object')
missing.unlink()
failure=subprocess.run([os.environ['S100_REAL_DU'],'-sb',str(missing)],capture_output=True)
if mode in ['permission','mixed']:
    blocked=pathlib.Path(sys.argv[-1])/'blocked'
    blocked.mkdir()
    blocked.chmod(0)
    try:
        denied=subprocess.run([os.environ['S100_REAL_DU'],'-sb',str(blocked)],capture_output=True)
    finally:
        blocked.chmod(0o700)
    assert denied.returncode!=0, 'permission negative control unavailable'
    sys.stderr.buffer.write(denied.stderr)
if mode!='permission':
    sys.stderr.buffer.write(failure.stderr)
sys.exit(1)
"""


@unittest.skipUnless(sys.platform == 'linux' and DU, 'Linux real-du metadata control')
class ResourceMeasurement(unittest.TestCase):
    """Retry only confirmed disappeared own paths; all other diagnostics stay fatal."""

    def observe(self, directory, mode):
        source, task, tools = [directory / name for name in ['source', 'task', 'tools']]
        for path in [source, task, tools]:
            path.mkdir()
        shim = tools / 'du'
        shim.write_text('#!' + sys.executable + '\n' + SHIM)
        shim.chmod(0o700)
        counter = directory / 'attempts'
        missing = (directory / 'outside' if mode == 'outside' else task / 'target') / 'gone.o'
        env = dict(PATH=str(tools) + os.pathsep + os.environ['PATH'], LC_ALL='C', S100_REAL_DU=DU,
                   S100_DU_COUNTER=str(counter), S100_DU_MODE=mode, S100_DU_MISSING=str(missing))
        return source, task, counter, env

    def test_confirmed_owned_disappearance_gets_fresh_measurement(self):
        with tempfile.TemporaryDirectory(prefix='s100-du-') as tmp:
            source, task, counter, env = self.observe(Path(tmp), 'once')
            with patch.dict(os.environ, env):
                result = control.resource_sample(source, task)
            self.assertGreater(result['owned'], 0)
            self.assertEqual(int(counter.read_text()), 2)

    def test_disappearance_retry_exhaustion_stops(self):
        with tempfile.TemporaryDirectory(prefix='s100-du-') as tmp:
            source, task, counter, env = self.observe(Path(tmp), 'always')
            with patch.dict(os.environ, env), self.assertRaises(control.Unqualified):
                control.resource_sample(source, task)
            self.assertEqual(int(counter.read_text()), 3)

    def test_outside_permission_and_mixed_errors_never_retry(self):
        for mode in ['outside', 'permission', 'mixed']:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(prefix='s100-du-') as tmp:
                source, task, counter, env = self.observe(Path(tmp), mode)
                with patch.dict(os.environ, env), self.assertRaises(control.Unqualified):
                    control.resource_sample(source, task)
                self.assertEqual(int(counter.read_text()), 1)


if __name__ == '__main__':
    unittest.main()
