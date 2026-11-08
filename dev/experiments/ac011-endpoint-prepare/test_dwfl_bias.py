"""Exercise the real preload ABI against a tiny downstream library, never a database."""

import os
import shlex
import subprocess
import unittest
from pathlib import Path


class DwflBiasTests(unittest.TestCase):
    """The exact configured call is corrected; all other calls are forwarded."""

    @classmethod
    def setUpClass(cls):
        source = Path(__file__).resolve().parent
        cls.output = Path(os.environ.get('DWFL_BIAS_TEST_OUTPUT',
                                        source.parents[2] / 'target' / 'dwfl-bias-test'))
        cls.output.mkdir(parents=True, exist_ok=True)
        cc = shlex.split(os.environ.get('CC', 'cc'))
        flags = ['-std=c11', '-Wall', '-Wextra', '-Werror']
        subprocess.run([*cc, *flags, '-fPIC', '-shared', '-DBUILD_NEXT',
                        str(source / 'test_dwfl_bias.c'), '-o',
                        str(cls.output / 'libdwfl_next.so')], check=True)
        subprocess.run([*cc, *flags, str(source / 'test_dwfl_bias.c'),
                        '-L' + str(cls.output), '-Wl,-rpath,$ORIGIN', '-ldwfl_next',
                        '-o', str(cls.output / 'driver')], check=True)
        subprocess.run([*cc, *flags, '-fPIC', '-shared', str(source / 'dwfl_bias.c'),
                        '-ldl', '-o', str(cls.output / 'libdwfl_bias.so')], check=True)

    def test_exact_correction_passthrough_arguments_return_and_audit_count(self):
        target = '/fixture/canonical-perf'
        other = '/fixture/other-dso.so'
        near = target + '-suffix'
        audit = self.output / 'corrections.audit'
        audit.write_text('')
        env = dict(os.environ, LD_PRELOAD=str(self.output / 'libdwfl_bias.so'),
                   FATHOMDB_DWFL_TARGET=target, FATHOMDB_DWFL_BAD_BASE='0x5000',
                   FATHOMDB_DWFL_CORRECT_BASE='0x4000', FATHOMDB_DWFL_AUDIT=str(audit))
        result = subprocess.run([str(self.output / 'driver'), target, other, near],
                                env=env, text=True, capture_output=True, check=True)
        rows = [line.split(maxsplit=4) for line in result.stdout.splitlines()]
        self.assertEqual([int(row[0], 16) for row in rows],
                         [0x4000, 0x5000, 0x5000, 0x6000, 0x5000, 0x4000, 0x4000])
        self.assertEqual([int(row[1]) for row in rows], [0, 0, 0, 0, 1, 0, 0])
        self.assertEqual([row[2:4] for row in rows], [['77', 'stable-module']] * 7)
        self.assertEqual([row[4] for row in rows], [target, other, near, target, target, target, target])
        self.assertEqual(result.stderr, '')
        corrections = audit.read_text().splitlines()
        self.assertEqual(len(corrections), 2)
        self.assertTrue(corrections[0].startswith('count=1 '))
        self.assertTrue(corrections[1].startswith('count=2 '))


if __name__ == '__main__':
    unittest.main()
