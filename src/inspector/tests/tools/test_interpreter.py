"""The checkout interpreter must work even with an unusable ambient Python."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class InterpreterTests(unittest.TestCase):
    def test_release_and_ci_tests_use_the_current_interpreter(self):
        with tempfile.TemporaryDirectory(prefix='inspector-old-python-') as directory:
            old_python = Path(directory) / 'python3'
            old_python.write_text('#!/bin/sh\necho ambient-python-is-not-the-checkout-interpreter >&2\nexit 86\n')
            old_python.chmod(0o755)
            env = dict(os.environ, PATH=directory + ':' + os.environ['PATH'])
            result = subprocess.run([sys.executable, '-m', 'unittest', 'discover', '-s', str(Path(__file__).parent), '-p', 'test_local_*.py'], env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()
