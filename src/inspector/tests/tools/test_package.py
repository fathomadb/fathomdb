"""Package command filesystem/failure tests; real builds run separately in local CI."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='inspector-package-command-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        tools = self.root / 'src/inspector/tools'
        tools.mkdir(parents=True)
        self.script = tools / 'package.sh'
        shutil.copyfile(ROOT / 'src/inspector/tools/package.sh', self.script)
        package = self.root / 'src/inspector/package'
        package.mkdir()
        (package / 'README.md').write_text('Install locally')
        (self.root / 'LICENSE').write_text('Apache License')
        bins = self.root / 'bin'
        bins.mkdir()
        cargo = bins / 'cargo'
        cargo.write_text('#!' + sys.executable + '''
import os, pathlib, sys
if os.environ.get('FAIL_BUILD'):
    sys.exit(19)
target = pathlib.Path(sys.argv[sys.argv.index('--target-dir')+1])
binary = target / 'x86_64-unknown-linux-gnu/release/fathomdb-inspector'
binary.parent.mkdir(parents=True, exist_ok=True)
binary.write_text("#!/bin/sh\\nprintf '%s' '{\\\"cached_default_cpu\\\":true,\\\"fixture_control\\\":false}'\\n")
binary.chmod(0o755)
''')
        cargo.chmod(0o755)
        self.env = dict(os.environ, PATH=str(bins) + ':' + os.environ['PATH'])

    def test_archive_has_outer_checksum_and_respects_isolated_target(self):
        target = self.root / 'isolated-target'
        output = self.root / 'candidate'
        result = subprocess.run(['bash', str(self.script), str(output)], env=dict(self.env, CARGO_TARGET_DIR=str(target)), capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        archive = output / 'fathomdb-inspector-0.2.0-linux-x86_64.tar.gz'
        self.assertEqual((output / 'SHA256SUMS').read_text(), hashlib.sha256(archive.read_bytes()).hexdigest() + '  ' + archive.name + '\n')
        self.assertTrue((target / 'x86_64-unknown-linux-gnu/release/fathomdb-inspector').is_file())

    def test_failed_build_propagates_status_and_removes_only_new_output(self):
        output = self.root / 'candidate'
        sentinel = self.root / 'unrelated'
        sentinel.write_text('keep')
        result = subprocess.run(['bash', str(self.script), str(output)], env=dict(self.env, FAIL_BUILD='1'), capture_output=True, text=True)
        self.assertEqual(result.returncode, 19)
        self.assertFalse(output.exists(), 'failed build must not leave a partial candidate')
        self.assertEqual(sentinel.read_text(), 'keep')


if __name__ == '__main__':
    unittest.main()
