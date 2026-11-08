"""Check receiver-native module reuse through real dynamic interposition, not perf data."""

import os
import shlex
import subprocess
import unittest
from pathlib import Path


class DwflNativeReuseTests(unittest.TestCase):
    """Duplicate fd=-1 reports reuse only a confirmed existing native module."""

    @classmethod
    def setUpClass(cls):
        source = Path(__file__).resolve().parent
        cls.output = Path(os.environ.get('DWFL_REUSE_TEST_OUTPUT',
                                        source.parents[2] / 'target' / 'dwfl-bias-reuse-test'))
        cls.output.mkdir(parents=True, exist_ok=True)
        cc = shlex.split(os.environ.get('CC', 'cc'))
        flags = ['-std=c11', '-Wall', '-Wextra', '-Werror']
        subprocess.run([*cc, *flags, '-fPIC', '-shared', '-DBUILD_NEXT',
                        str(source / 'test_dwfl_bias_reuse.c'), '-o',
                        str(cls.output / 'libdwfl_receiver.so')], check=True)
        subprocess.run([*cc, *flags, str(source / 'test_dwfl_bias_reuse.c'),
                        '-L' + str(cls.output), '-Wl,-rpath,$ORIGIN', '-ldwfl_receiver',
                        '-o', str(cls.output / 'driver')], check=True)
        subprocess.run([*cc, *flags, '-fPIC', '-shared', str(source / 'dwfl_bias.c'),
                        '-ldl', '-o', str(cls.output / 'libdwfl_bias.so')], check=True)

    def test_existing_native_handle_skips_duplicate_report_only_when_confirmed(self):
        target = '/fixture/canonical-perf'
        audit = self.output / 'reuse.audit'
        audit.write_text('')
        env = dict(os.environ, LD_PRELOAD=str(self.output / 'libdwfl_bias.so'),
                   FATHOMDB_DWFL_TARGET=target, FATHOMDB_DWFL_BAD_BASE='0x5000',
                   FATHOMDB_DWFL_CORRECT_BASE='0x4000', FATHOMDB_DWFL_AUDIT=str(audit))
        result = subprocess.run([str(self.output / 'driver'), target], env=env,
                                text=True, capture_output=True, check=True)
        lines = result.stdout.splitlines()
        forwarded = [line for line in lines if line.startswith('next ')]
        self.assertEqual(len(forwarded), 5, 'confirmed native module must skip duplicate reporting')
        self.assertEqual(forwarded, ['next base=4000 fd=-1'] * 4 + ['next base=4000 fd=77'])
        self.assertEqual([line for line in lines if line.startswith('existing=')],
                         ['existing=0', 'existing=1', 'existing=0', 'existing=0',
                          'existing=0', 'existing=0'])
        self.assertEqual(result.stderr, '')
        records = [dict(field.split('=', 1) for field in line.split())
                   for line in audit.read_text().splitlines()]
        self.assertEqual([r['action'] for r in records],
                         ['registered', 'reused', 'registered', 'registered', 'registered', 'registered'])
        self.assertEqual([int(r['action_count']) for r in records], [1, 1, 2, 3, 4, 5])
        self.assertEqual([int(r['count']) for r in records], [1, 2, 3, 4, 5, 6])


if __name__ == '__main__':
    unittest.main()
