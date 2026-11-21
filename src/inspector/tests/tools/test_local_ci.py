"""Local CI composition: explicit prerequisites and source-stage failure status."""
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

    def test_missing_fixture_prerequisite_is_actionable_failure(self):
        result = subprocess.run(['bash', str(self.script), str(self.base / 'candidate'), str(self.base / 'absent'), str(self.base)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('prerequisite:', result.stderr)
        self.assertIn('fixtures', result.stderr)
        self.assertFalse((self.base / 'candidate').exists())

    def test_source_failure_stops_before_packaging_and_preserves_exit(self):
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
        (venv / 'python').symlink_to(sys.executable)
        bins = self.base / 'bin'
        bins.mkdir()
        for name, content in {'docker': '#!/bin/sh\nexit 0\n', 'git': '#!/bin/sh\nexit 0\n'}.items():
            (bins / name).write_text(content)
            (bins / name).chmod(0o755)
        browser = self.base / 'browser'
        browser.mkdir()
        (browser / 'chrome').write_text('test prerequisite only')
        (browser / 'chrome').chmod(0o755)
        modules = self.base / 'modules/playwright'
        modules.mkdir(parents=True)
        (modules / 'index.mjs').write_text('')
        env = dict(os.environ, PATH=str(bins) + ':' + os.environ['PATH'],
                   INSPECTOR_CHROMIUM_DIRECTORY=str(browser), INSPECTOR_NODE_MODULES=str(modules.parent))
        result = subprocess.run(['bash', str(self.script), str(self.base / 'candidate'), str(fixtures), str(self.base)], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 37, result.stderr)
        self.assertIn('stage=source', result.stderr)
        self.assertFalse((self.base / 'candidate').exists())

    def test_unusable_checkout_python_fails_before_source(self):
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
        (venv / 'python').write_text('#!/bin/sh\nexit 49\n')
        (venv / 'python').chmod(0o755)
        bins = self.base / 'bin'
        bins.mkdir()
        for name, content in {'docker': '#!/bin/sh\nexit 0\n', 'git': '#!/bin/sh\nexit 0\n'}.items():
            (bins / name).write_text(content)
            (bins / name).chmod(0o755)
        browser = self.base / 'browser'
        browser.mkdir()
        (browser / 'chrome').write_text('test prerequisite only')
        (browser / 'chrome').chmod(0o755)
        modules = self.base / 'modules/playwright'
        modules.mkdir(parents=True)
        (modules / 'index.mjs').write_text('')
        env = dict(os.environ, PATH=str(bins) + ':' + os.environ['PATH'],
                   INSPECTOR_CHROMIUM_DIRECTORY=str(browser), INSPECTOR_NODE_MODULES=str(modules.parent))
        result = subprocess.run(['bash', str(self.script), str(self.base / 'candidate'), str(fixtures), str(self.base)], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('prerequisite: checkout Python', result.stderr)
        self.assertFalse((self.base / 'candidate').exists())


if __name__ == '__main__':
    unittest.main()
