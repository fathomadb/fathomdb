"""Harmless real stdlib-child cleanup controls; no GDB, product, ptrace or fault case."""

import importlib.util
from pathlib import Path
import select
import subprocess
import sys
import time
import unittest

SPEC = importlib.util.spec_from_file_location("slice100_operator_cleanup", Path(__file__).with_name("operator.py"))
control = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(control)


@unittest.skipUnless(sys.platform == "linux", "Linux owned-process controller; no Windows qualification")
class OwnedHelperCleanup(unittest.TestCase):
    """Use only helpers created by this test; never inspect or signal other actors."""

    def helper(self):
        child = subprocess.Popen([sys.executable, '-u', '-c', 'import time; print("READY", flush=True); time.sleep(60)'], stdout=subprocess.PIPE)
        self.addCleanup(self.reap_helper, child)
        self.assertTrue(select.select([child.stdout], [], [], 3)[0], 'helper readiness timeout')
        self.assertEqual(child.stdout.readline(), b'READY\n')
        return child

    def reap_helper(self, child):
        if child.poll() is None:
            child.kill()
        child.wait(timeout=5)
        child.stdout.close()

    def test_registered_helper_is_terminated_and_reaped_within_bound(self):
        child = self.helper()
        actors = control.Actors()
        actors.processes.append(child)
        started = time.monotonic()
        self.assertEqual(actors.cleanup(), [])
        self.assertLess(time.monotonic() - started, 5)
        self.assertIsNotNone(child.poll())

    def test_controller_never_signals_second_helper_outside_its_registry(self):
        registered = self.helper()
        second = self.helper()
        actors = control.Actors()
        actors.processes.append(registered)
        self.assertEqual(actors.cleanup(), [])
        self.assertIsNone(second.poll(), 'cleanup escaped its registered actor set')


if __name__ == '__main__':
    unittest.main()
