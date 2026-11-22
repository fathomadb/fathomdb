"""Local CI composition: explicit native Ubuntu 24.04 prerequisites and source-stage failure status."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]


class LocalCiTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='inspector-local-ci-')
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        tools = self.base / 'src/inspector/tools'
        tools.mkdir(parents=True)
        self.script = tools / 'local-ci.sh'
        original = ROOT / 'src/inspector/tools/local-ci.sh'
        if original.exists():
            shutil.copyfile(original, self.script)

    def _prepare_native_env(self, python_setup):
        fixtures = self.base / 'fixtures'
        for fixture in ['person', 'vector', 'f06', 'study-training', 'study-unseen', 'lexical-deadline']:
            (fixtures / fixture / 'work').mkdir(parents=True)
            (fixtures / fixture / 'work/fixture.sqlite').write_text('not opened by this composition test')
        scripts = self.base / 'scripts'
        scripts.mkdir()
        (scripts / 'agent-verify.sh').write_text('#!/bin/sh\nexit 37\n')
        (scripts / 'agent-verify.sh').chmod(0o755)
        venv = self.base / '.venv/bin'
        venv.mkdir(parents=True)
        python_setup(venv / 'python')
        bins = self.base / 'bin'
        bins.mkdir()
        # Deliberately poison docker to verify native local-ci.sh never requires or invokes Docker.
        for name, content in {
            'docker': '#!/bin/sh\necho "unexpected docker call" >&2\nexit 99\n',
            'git': '#!/bin/sh\nexit 0\n',
            'Xvfb': '#!/bin/sh\nexit 0\n',
            'flock': '#!/bin/sh\nexit 0\n',
            'readelf': '#!/bin/sh\nexit 0\n',
        }.items():
            (bins / name).write_text(content)
            (bins / name).chmod(0o755)
        browser = self.base / 'browser'
        browser.mkdir()
        (browser / 'chrome').write_text('test prerequisite only')
        (browser / 'chrome').chmod(0o755)
        modules = self.base / 'modules/playwright'
        modules.mkdir(parents=True)
        (modules / 'index.mjs').write_text('')
        env = dict(
            os.environ,
            PATH=str(bins) + ':' + os.environ['PATH'],
            INSPECTOR_CHROMIUM_DIRECTORY=str(browser),
            INSPECTOR_NODE_MODULES=str(modules.parent),
        )
        return fixtures, env

    def test_missing_fixture_prerequisite_is_actionable_failure(self):
        result = subprocess.run(['bash', str(self.script), str(self.base / 'candidate'), str(self.base / 'absent'), str(self.base)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('prerequisite:', result.stderr)
        self.assertIn('fixtures', result.stderr)
        self.assertFalse((self.base / 'candidate').exists())

    def test_source_failure_stops_before_packaging_without_docker_and_preserves_exit(self):
        fixtures, env = self._prepare_native_env(lambda p: p.symlink_to(sys.executable))
        result = subprocess.run(['bash', str(self.script), str(self.base / 'candidate'), str(fixtures), str(self.base)], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 37, result.stderr)
        self.assertIn('stage=source', result.stderr)
        self.assertNotIn('docker', result.stderr.lower())
        self.assertFalse((self.base / 'candidate').exists())

    def test_unusable_checkout_python_fails_before_source(self):
        def bad_python(path):
            path.write_text('#!/bin/sh\nexit 49\n')
            path.chmod(0o755)

        fixtures, env = self._prepare_native_env(bad_python)
        result = subprocess.run(['bash', str(self.script), str(self.base / 'candidate'), str(fixtures), str(self.base)], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('prerequisite: checkout Python', result.stderr)
        self.assertFalse((self.base / 'candidate').exists())


if __name__ == '__main__':
    unittest.main()
