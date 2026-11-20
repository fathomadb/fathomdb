#!/usr/bin/env python3
"""Exercise the registry consumer's checks without Cargo, CUDA or network."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / 'check-tegra-pool-registry-build.sh'
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
COHORT = {
    'cudarc-fathomdb': '0.21.0',
    'candle-core-fathomdb': '0.13.0',
    'candle-nn-fathomdb': '0.13.0',
    'candle-transformers-fathomdb': '0.13.0',
    'candle-kernels-fathomdb': '0.13.0',
}


class RegistryConsumer(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='registry-consumer-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'scripts').mkdir()
        self.script = self.root / 'scripts' / SCRIPT.name
        shutil.copy(SCRIPT, self.script)
        tests = self.root / 'src/rust/crates/fathomdb-embedder/tests'
        (tests / 'support').mkdir(parents=True)
        for path in ('tegra_fragmented_va_cuda.rs', 'support/live.rs'):
            (tests / path).write_text('// Fixture; never compiled.\n')
        for path in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'):
            (self.root / path).write_text('# Fixture\n')
        for args in (('init', '--quiet'), ('add', '.')):
            subprocess.run(['git', '-C', str(self.root), *args], check=True)
        (self.root / 'bin').mkdir()
        cargo = self.root / 'bin/cargo'
        cargo.write_text('''#!/usr/bin/env bash
set -euo pipefail
[ "$*" = "generate-lockfile -q" ] || exit 90
cp "$REGISTRY_LOCK_FIXTURE" Cargo.lock
cp Cargo.toml "$MANIFEST_CAPTURE"
''')
        cargo.chmod(0o755)
        (self.root / 'tmp').mkdir()
        self.capture = self.root / 'consumer-manifest'

    def check(self, packages=None):
        if packages is None:
            packages = [(name, version, REGISTRY) for name, version in COHORT.items()]
        lock = self.root / 'input.lock'
        lock.write_text('version = 3\n' + ''.join(
            f'\n[[package]]\nname = "{name}"\nversion = "{version}"\n' +
            (f'source = "{source}"\n' if source is not None else '')
            for name, version, source in packages))
        env = dict(os.environ, PATH=f'{self.root}/bin:' + os.environ['PATH'],
                   TMPDIR=str(self.root / 'tmp'), REGISTRY_LOCK_FIXTURE=str(lock),
                   MANIFEST_CAPTURE=str(self.capture))
        return subprocess.run(['bash', str(self.script), '--resolve-only'],
                              env=env, capture_output=True, text=True)

    def test_current_registry_cohort_pass(self):
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('--resolve-only: (b) and (c) not run', result.stdout)
        manifest = self.capture.read_text()
        self.assertIn('package = "candle-core-fathomdb", version = "=0.13.0"', manifest)
        self.assertNotIn('[patch', manifest)

    def test_old_cohort_fails(self):
        packages = [(name, '0.19.10' if name == 'cudarc-fathomdb' else
                     '0.10.2' if name == 'candle-kernels-fathomdb' else '0.11.0', REGISTRY)
                    for name in COHORT]
        result = self.check(packages)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('expected exactly 0.21.0 from crates.io', result.stderr)

    def test_wrong_versions_fail(self):
        for wrong in COHORT:
            with self.subTest(package=wrong):
                packages = [(name, '99.0.0' if name == wrong else version, REGISTRY)
                            for name, version in COHORT.items()]
                result = self.check(packages)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f'{wrong}: expected exactly {COHORT[wrong]} from crates.io', result.stderr)

    def test_nonregistry_sources_fail(self):
        for wrong in COHORT:
            for source in ('git+https://example.invalid/fork#' + 'a' * 40, None):
                with self.subTest(package=wrong, source=source):
                    packages = [(name, version, source if name == wrong else REGISTRY)
                                for name, version in COHORT.items()]
                    result = self.check(packages)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(f'{wrong}: expected exactly', result.stderr)

    def test_duplicate_fork_fails(self):
        packages = [(name, version, REGISTRY) for name, version in COHORT.items()]
        packages.append(packages[0])
        result = self.check(packages)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cudarc-fathomdb: expected exactly', result.stderr)

    def test_upstream_current_and_previous_cudarc_families_fail(self):
        for version in ('0.17.8', '0.19.10', '0.20.0', '0.21.0'):
            with self.subTest(version=version):
                packages = [(name, value, REGISTRY) for name, value in COHORT.items()]
                packages.append(('cudarc', version, REGISTRY))
                result = self.check(packages)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f'upstream cudarc {version} resolved', result.stderr)

    def test_upstream_kernel_fails(self):
        packages = [(name, value, REGISTRY) for name, value in COHORT.items()]
        packages.append(('candle-kernels', '0.10.2', REGISTRY))
        result = self.check(packages)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('upstream candle-kernels 0.10.2 resolved', result.stderr)

    def test_consumer_patch_fails_before_cargo(self):
        original = self.script.read_text()
        marker = '[workspace]\n\n[features]'
        self.assertIn(marker, original)
        self.script.write_text(original.replace(marker, '[workspace]\n\n[patch.crates-io]\n\n[features]', 1))
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('the consumer manifest has a [patch] table', result.stderr)
        self.assertFalse(self.capture.exists(), 'Cargo must not run with a patched consumer')


if __name__ == '__main__':
    unittest.main()
