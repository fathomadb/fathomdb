#!/usr/bin/env python3
"""Current-main ancestry on real local Git histories, without publication."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from test_ci_proportional_routing import Expression, block_scalar, job_block, normalized_condition

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


class PagesAuthorization(unittest.TestCase):
    def setUp(self):
        self.workflow = (HELPER.parents[2] / '.github/workflows/docs-pages.yml').read_text()
        self.authorization = job_block(self.workflow, 'authorize-pages')
        self.assertIsNotNone(self.authorization, 'Pages needs an early publication authorization job')
        self.script = block_scalar(self.authorization, 'run', 8)
        self.assertIsNotNone(self.script, 'authorization must execute its release-state check')
        self.temp = tempfile.TemporaryDirectory(prefix='pages-authorization-')
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.git('init', '-b', 'main')
        self.git('config', 'user.name', 'Fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        (self.repo / 'scripts').mkdir()
        for script in ('release-current.py', 'check-public-doc-truth.py'):
            shutil.copy(HELPER.parents[1] / script, self.repo / 'scripts' / script)
        self.state_path = self.repo / 'dev/plans/release-state-9.9.9.json'
        self.board = self.repo / 'dev/plans/runs/STATUS-9.9.9.md'
        self.board.parent.mkdir(parents=True)
        self.board.write_text('# Active release\n')
        self.state = {'release': '9.9.9', 'board': 'dev/plans/runs/STATUS-9.9.9.md'}

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.repo), *args], stderr=subprocess.DEVNULL, text=True).strip()

    def run_authorization(self, candidate=None):
        self.state_path.write_text(json.dumps(self.state))
        self.git('add', '.')
        self.git('commit', '--allow-empty', '-m', 'release fixture')
        output = self.repo / 'github-output'
        output.write_text('')
        env = dict(os.environ, GITHUB_OUTPUT=str(output), GITHUB_SHA=candidate or self.git('rev-parse', 'HEAD'))
        result = subprocess.run(['bash', '-c', self.script], cwd=self.repo, env=env, capture_output=True, text=True)
        outputs = dict(line.split('=', 1) for line in output.read_text().splitlines())
        return result, outputs

    def test_only_json_boolean_true_authorizes(self):
        for value in (True, False, 'true', 'false', 1, 0, None, [], {}):
            with self.subTest(value=value):
                self.state['release_readiness'] = {'pages_publication_authorized': value}
                result, outputs = self.run_authorization()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(outputs.get('authorized'), 'true' if value is True else 'false')

    def test_missing_authorization_skips_cleanly(self):
        for readiness in (None, {}):
            with self.subTest(readiness=readiness):
                if readiness is None:
                    self.state.pop('release_readiness', None)
                else:
                    self.state['release_readiness'] = readiness
                result, outputs = self.run_authorization()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(outputs.get('authorized'), 'false')

    def published_receipt(self, version='9.9.9'):
        return {'tag': f'v{version}', 'tag_commit': 'a' * 40,
                'published_on': '2026-11-17', 'npm_dist_tag': 'latest'}

    def add_published_state(self, version, authorization):
        board = f'dev/plans/runs/STATUS-{version}.md'
        (self.repo / board).write_text('# Published release\n')
        state = {'release': version, 'board': board,
                 'published': self.published_receipt(version),
                 'release_readiness': {'pages_publication_authorized': authorization}}
        (self.repo / f'dev/plans/release-state-{version}.json').write_text(json.dumps(state))

    def test_no_active_release_skips_cleanly_without_authorization(self):
        self.state['published'] = self.published_receipt()
        for readiness in (None, {}, {'pages_publication_authorized': False}):
            with self.subTest(readiness=readiness):
                if readiness is None:
                    self.state.pop('release_readiness', None)
                else:
                    self.state['release_readiness'] = readiness
                result, outputs = self.run_authorization()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(outputs.get('authorized'), 'false')

    def test_no_active_release_only_json_boolean_true_authorizes(self):
        self.state['published'] = self.published_receipt()
        for value in (True, False, 'true', 'false', 1, 0, None, [], {}):
            with self.subTest(value=value):
                self.state['release_readiness'] = {'pages_publication_authorized': value}
                result, outputs = self.run_authorization()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(outputs.get('authorized'), 'true' if value is True else 'false')

    def test_newest_published_state_controls_authorization(self):
        self.state['published'] = self.published_receipt()
        self.add_published_state('8.8.8', True)
        self.state['release_readiness'] = {'pages_publication_authorized': False}
        result, outputs = self.run_authorization()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(outputs.get('authorized'), 'false')
        self.state['release_readiness'] = {'pages_publication_authorized': True}
        self.add_published_state('8.8.8', False)
        result, outputs = self.run_authorization()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(outputs.get('authorized'), 'true')

    def test_active_release_cannot_fall_back_to_historical_authorization(self):
        self.add_published_state('8.8.8', True)
        for readiness in ({}, {'pages_publication_authorized': False}):
            with self.subTest(readiness=readiness):
                self.state['release_readiness'] = readiness
                result, outputs = self.run_authorization()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(outputs.get('authorized'), 'false')

    def test_malformed_published_receipt_fails_closed(self):
        self.state['release_readiness'] = {'pages_publication_authorized': True}
        for receipt in ({}, 'true', self.published_receipt('8.8.8'),
                        dict(self.published_receipt(), tag_commit='broken'),
                        dict(self.published_receipt(), published_on='2026-02-30')):
            with self.subTest(receipt=receipt):
                self.state['published'] = receipt
                result, outputs = self.run_authorization()
                self.assertNotEqual(result.returncode, 0)
                self.assertNotEqual(outputs.get('authorized'), 'true')

    def test_no_published_receipt_cannot_authorize_historical_state(self):
        self.board.write_text('# Closed — historical record\n')
        self.state['release_readiness'] = {'pages_publication_authorized': True}
        result, outputs = self.run_authorization()
        self.assertNotEqual(outputs.get('authorized'), 'true')

    def test_published_state_wrong_event_candidate_fails_closed(self):
        self.state['published'] = self.published_receipt()
        self.state['release_readiness'] = {'pages_publication_authorized': True}
        result, outputs = self.run_authorization(candidate='f' * 40)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotEqual(outputs.get('authorized'), 'true')

    def test_selector_and_malformed_state_fail_closed(self):
        self.state['release_readiness'] = {'pages_publication_authorized': True}
        for content in ('{', json.dumps({'release': 'broken'})):
            with self.subTest(content=content):
                (self.repo / 'dev/plans/release-state-8.8.8.json').write_text(content)
                result, outputs = self.run_authorization()
                self.assertNotEqual(result.returncode, 0)
                self.assertNotEqual(outputs.get('authorized'), 'true')

    def test_wrong_event_candidate_fails_closed(self):
        self.state['release_readiness'] = {'pages_publication_authorized': True}
        result, outputs = self.run_authorization(candidate='f' * 40)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotEqual(outputs.get('authorized'), 'true')

    def test_workflow_requires_success_and_exact_authorization(self):
        self.assertIn('runs-on: ubuntu-latest', self.authorization)
        self.assertIn('ref: ${{ github.sha }}', self.authorization)
        self.assertIn('persist-credentials: false', self.authorization)
        self.assertIn('contents: read', self.authorization)
        self.assertNotIn(': write', self.authorization)
        self.assertIn('authorized: ${{ steps.authorization.outputs.authorized }}', self.authorization)
        self.assertIn('id: authorization', self.authorization)
        prepare = job_block(self.workflow, 'prepare-pages')
        self.assertIn('needs: authorize-pages', prepare)
        condition = normalized_condition(prepare)
        self.assertIsNotNone(condition)
        self.assertNotIn('always()', condition)
        for result in ('success', 'failure', 'cancelled', 'skipped'):
            for authorized in ('true', 'false', '', 'True', '1'):
                context = {'needs.authorize-pages.result': result,
                           'needs.authorize-pages.outputs.authorized': authorized}
                self.assertEqual(Expression(condition, context).evaluate(),
                                 result == 'success' and authorized == 'true', context)
        for job, predecessor in (('deploy-pages', 'prepare-pages'), ('verify-pages-live', 'deploy-pages')):
            block = job_block(self.workflow, job)
            self.assertIn('needs: ' + predecessor, block)
            self.assertIsNone(normalized_condition(block), 'publication chain must depend on predecessor success')
        self.assertEqual(self.workflow.count('scripts/release/check-pages-source.py'), 3)


if __name__ == '__main__':
    unittest.main()
