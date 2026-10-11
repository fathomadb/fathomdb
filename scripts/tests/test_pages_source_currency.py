#!/usr/bin/env python3
"""Current-main ancestry on real local Git histories, without publication."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HELPER = Path(__file__).resolve().parents[1] / 'release/check-pages-source.py'


class PagesCurrency(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='slice140-pages-history-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.remote = self.root / 'remote.git'
        self.writer = self.root / 'writer'
        self.client = self.root / 'client'
        self.git(self.root, 'init', '--bare', str(self.remote))
        self.git(self.root, 'init', '-b', 'main', str(self.writer))
        self.git(self.writer, 'config', 'user.name', 'Fixture')
        self.git(self.writer, 'config', 'user.email', 'fixture@example.invalid')
        self.git(self.writer, 'remote', 'add', 'origin', str(self.remote))
        self.old = self.commit('old')
        self.main = self.commit('main')
        self.git(self.writer, 'push', 'origin', 'main')
        self.git(self.writer, 'checkout', '-b', 'release/current')
        self.ahead = self.commit('ahead')
        self.git(self.writer, 'push', 'origin', 'release/current')
        self.git(self.writer, 'checkout', '-b', 'release/stale', self.old)
        self.diverged = self.commit('diverged')
        self.git(self.writer, 'push', 'origin', 'release/stale')
        self.git(self.root, 'clone', '-b', 'release/current', str(self.remote), str(self.client))

    def git(self, cwd, *args):
        return subprocess.check_output(['git', '-C', str(cwd), *args], stderr=subprocess.DEVNULL, text=True).strip()

    def commit(self, text):
        (self.writer / 'source').write_text(text)
        self.git(self.writer, 'add', 'source')
        self.git(self.writer, 'commit', '-m', text)
        return self.git(self.writer, 'rev-parse', 'HEAD')

    def check(self, candidate, repo=None):
        return subprocess.run([sys.executable, str(HELPER), '--repo', str(repo or self.client), '--candidate', candidate], capture_output=True, text=True)

    def test_equal_and_ahead_pass(self):
        for sha in (self.main, self.ahead):
            result = self.check(sha)
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_behind_and_diverged_fail(self):
        for sha in (self.old, self.diverged):
            result = self.check(sha)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('does not contain current origin/main', result.stderr)

    def test_main_advance_is_refetched_for_queued_deploy(self):
        self.assertEqual(self.check(self.ahead).returncode, 0)
        self.git(self.writer, 'checkout', 'main')
        self.commit('new main after build')
        self.git(self.writer, 'push', 'origin', 'main')
        result = self.check(self.ahead)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('does not contain current origin/main', result.stderr)

    def test_missing_candidate_ref_and_fetch_fail_closed(self):
        result = self.check('f' * 40)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('candidate commit is absent', result.stderr)
        self.git(self.client, 'remote', 'set-url', 'origin', str(self.root / 'absent.git'))
        result = self.check(self.ahead)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cannot fetch current origin/main', result.stderr)

    def test_missing_main_and_shallow_history_fail_closed(self):
        self.git(self.remote, 'update-ref', '-d', 'refs/heads/main')
        result = self.check(self.ahead)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cannot fetch current origin/main', result.stderr)
        self.git(self.writer, 'push', 'origin', 'main')
        shallow = self.root / 'shallow'
        self.git(self.root, 'clone', '--depth', '1', '-b', 'release/current', self.remote.as_uri(), str(shallow))
        result = self.check(self.ahead, shallow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('full Git history required', result.stderr)


if __name__ == '__main__':
    unittest.main()
