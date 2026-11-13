"""Documented CLI entrypoint smoke; --help cannot launch GDB or product code."""

from pathlib import Path
import subprocess
import sys
import unittest


class CLIEntry(unittest.TestCase):
    """The documented plain Python invocation must reach its parser without imports failing."""

    def test_documented_help_reaches_parser_without_technical_execution(self):
        result = subprocess.run([sys.executable, str(Path(__file__).with_name('operator.py')), '--help'],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('--technical-go', result.stdout)
        self.assertIn('--mode', result.stdout)


if __name__ == '__main__':
    unittest.main()
