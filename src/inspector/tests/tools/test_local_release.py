"""Local filesystem release contract; never substitutes for real artifact qualification."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[4]
TOOL = ROOT / 'src/inspector/tools/local-release.py'
NAME = 'fathomdb-inspector-0.2.1-linux-x86_64'
FILES = ['fathomdb-inspector', 'build-description.json', 'LICENSE', 'README.md']


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix='inspector-release-test-')
        self.addCleanup(self.scratch.cleanup)
        self.base = Path(self.scratch.name)
        self.candidate = self.base / 'candidate'
        self.candidate.mkdir()
        bundle = self.candidate / NAME
        bundle.mkdir()
        description = {'version': '0.2.1', 'build': 'a' * 64, 'target': 'x86_64-unknown-linux-gnu',
                       'profile': 'release', 'compiled_schema': 37, 'fixture_control': False,
                       'cached_default_cpu': True, 'protocol': 1}
        (bundle / 'build-description.json').write_text(json.dumps(description))
        (bundle / 'fathomdb-inspector').write_text('#!/bin/sh\ncat "$(dirname "$0")/build-description.json"\n')
        (bundle / 'fathomdb-inspector').chmod(0o755)
        (bundle / 'LICENSE').write_text('Apache License 2.0\n')
        (bundle / 'README.md').write_text('Local install instructions\n')
        (bundle / 'SHA256SUMS').write_text(''.join(f'{digest(bundle / f)}  {f}\n' for f in FILES))
        archive = self.candidate / f'{NAME}.tar.gz'
        with tarfile.open(archive, 'w:gz') as tar:
            tar.add(bundle, arcname=NAME)
        (self.candidate / 'SHA256SUMS').write_text(f'{digest(archive)}  {archive.name}\n')
        manifest = {'version': '0.2.1', 'source_sha': 'b' * 40, 'build': description,
                    'engine_commit': '72fe93634321616214352a7e07066ce0f93bcf4c',
                    'archive_sha256': digest(archive),
                    'executable_sha256': digest(bundle / 'fathomdb-inspector')}
        (self.candidate / 'candidate.json').write_text(json.dumps(manifest))

    def run_tool(self, *args):
        return subprocess.run([sys.executable, str(TOOL), *map(str, args)], capture_output=True, text=True)

    def test_verify_accepts_complete_candidate_and_rejects_changed_bytes(self):
        result = self.run_tool('verify', self.candidate)
        self.assertEqual(result.returncode, 0, result.stderr)
        (self.candidate / NAME / 'README.md').write_text('changed')
        result = self.run_tool('verify', self.candidate)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('checksum', result.stderr)

    def test_verify_rejects_archive_extras_and_wrong_production_features(self):
        (self.candidate / 'unexpected').write_text('unrelated')
        self.assertNotEqual(self.run_tool('verify', self.candidate).returncode, 0)
        (self.candidate / 'unexpected').unlink()
        manifest = self.candidate / 'candidate.json'
        value = json.loads(manifest.read_text())
        value['build']['fixture_control'] = True
        manifest.write_text(json.dumps(value))
        self.assertNotEqual(self.run_tool('verify', self.candidate).returncode, 0)

    def test_archive_without_executable_mode_is_rejected(self):
        archive = self.candidate / f'{NAME}.tar.gz'
        bundle = self.candidate / NAME
        def remove_executable_mode(info):
            if info.name == f'{NAME}/fathomdb-inspector':
                info.mode = 0o644
            return info
        with tarfile.open(archive, 'w:gz') as tar:
            tar.add(bundle, arcname=NAME, filter=remove_executable_mode)
        (self.candidate / 'SHA256SUMS').write_text(f'{digest(archive)}  {archive.name}\n')
        manifest = self.candidate / 'candidate.json'
        value = json.loads(manifest.read_text())
        value['archive_sha256'] = digest(archive)
        manifest.write_text(json.dumps(value))
        result = self.run_tool('verify', self.candidate)
        self.assertNotEqual(result.returncode, 0, 'extracted application must be executable')
        self.assertIn('mode', result.stderr)

    def test_promote_copies_verified_bytes_and_refuses_existing_destination(self):
        destination = self.base / 'release'
        result = self.run_tool('promote', self.candidate, destination)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((destination / 'candidate.json').read_bytes(), (self.candidate / 'candidate.json').read_bytes())
        self.assertEqual(digest(destination / NAME / 'fathomdb-inspector'), digest(self.candidate / NAME / 'fathomdb-inspector'))
        sentinel = destination / 'keep'
        sentinel.write_text('unrelated')
        self.assertNotEqual(self.run_tool('promote', self.candidate, destination).returncode, 0)
        self.assertEqual(sentinel.read_text(), 'unrelated')
        self.assertEqual(list(self.base.glob('.inspector-stage-*')), [])

    def test_interrupted_partial_copy_removes_only_owned_staging(self):
        spec = importlib.util.spec_from_file_location('local_release', TOOL)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        destination = self.base / 'release'
        sentinel = self.base / 'unrelated'
        sentinel.write_text('keep')

        def interrupt_copy(_source, stage, **_options):
            (stage / 'partial').write_text('partial')
            raise KeyboardInterrupt()

        with patch.object(module.shutil, 'copytree', side_effect=interrupt_copy):
            with self.assertRaises(KeyboardInterrupt):
                module.promote(self.candidate, destination)
        self.assertFalse(destination.exists())
        self.assertEqual(list(self.base.glob('.inspector-stage-*')), [])
        self.assertEqual(sentinel.read_text(), 'keep')

    def test_sigterm_during_copy_cleans_its_owned_staging(self):
        destination = self.base / 'release'
        ready = self.base / 'ready'
        code = """
import importlib.util, pathlib, sys, time
spec = importlib.util.spec_from_file_location('release', sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
def wait_during_copy(_source, stage, **_options):
    (stage / 'partial').write_text('partial')
    pathlib.Path(sys.argv[4]).write_text('ready')
    time.sleep(30)
module.shutil.copytree = wait_during_copy
module.promote(pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3]))
"""
        process = subprocess.Popen([sys.executable, '-c', code, str(TOOL), str(self.candidate), str(destination), str(ready)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 5
            while not ready.exists() and process.poll() is None and time.monotonic() < deadline:
                time.sleep(0.01)
            self.assertTrue(ready.exists(), 'test-owned copier must reach partial stage')
            process.terminate()
            process.communicate(timeout=5)
            self.assertFalse(destination.exists())
            self.assertEqual(list(self.base.glob('.inspector-stage-*')), [], 'SIGTERM must remove partial staging')
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate(timeout=5)


if __name__ == '__main__':
    unittest.main()
