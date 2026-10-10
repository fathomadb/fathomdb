#!/usr/bin/env python3
"""Source-derived documentation and whole-site publication contracts."""
from pathlib import Path
import json
import shutil
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
GENERATOR = REPO / 'scripts/generate-public-docs.py'
OUTPUTS = ('sdk-operation-map.md', 'contributor-map.md')
VERBS = ('build', 'lint', 'typecheck', 'test', 'verify')


class GeneratedDocs(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='slice140-generated-docs-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        inputs = ['Cargo.toml', 'src/conformance/governed-operation-parity.json']
        inputs += [p.relative_to(REPO).as_posix() for p in (REPO / 'src/rust/crates').glob('*/Cargo.toml')]
        inputs += [f'scripts/agent-{verb}.sh' for verb in VERBS]
        for name in inputs:
            dest = self.root / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(REPO / name, dest)

    def run_generator(self, mode):
        return subprocess.run([sys.executable, str(GENERATOR), '--root', str(self.root), mode], text=True, capture_output=True)

    def generate(self):
        result = self.run_generator('--write')
        self.assertEqual(result.returncode, 0, result.stderr)
        return [self.root / 'docs/generated' / name for name in OUTPUTS]

    def test_exact_current_operation_index_and_determinism(self):
        pages = self.generate()
        text = pages[0].read_text()
        operations = json.loads((self.root / 'src/conformance/governed-operation-parity.json').read_text())['operations']
        self.assertEqual(len(operations), 44)
        self.assertEqual(len([line for line in text.splitlines() if line.startswith('| `')]), 44)
        for operation in operations:
            row = next(line for line in text.splitlines() if line.startswith(f"| `{operation['id']}` |"))
            for language in ('python', 'typescript', 'rust'):
                self.assertIn(f"`{operation[language]['locator']}` / `{operation[language]['spelling']}`", row)
        self.assertIn('operation index', text)
        for name in ('python-api', 'typescript-api', 'rust-sdk'):
            self.assertIn(f'../reference/{name}.md', text)
        original = [p.read_bytes() for p in pages]
        self.assertEqual(self.run_generator('--check').returncode, 0)
        self.generate()
        self.assertEqual(original, [p.read_bytes() for p in pages])
        self.assertNotIn('0.8.26', pages[1].read_text())
        self.assertNotIn(str(self.root), '\n'.join(p.read_text() for p in pages))

    def test_changed_source_stale_check_is_read_only(self):
        pages = self.generate()
        before = [p.read_bytes() for p in pages]
        manifest = self.root / 'src/conformance/governed-operation-parity.json'
        value = json.loads(manifest.read_text())
        value['operations'][0]['typescript']['spelling'] = 'changedOpen'
        manifest.write_text(json.dumps(value))
        result = self.run_generator('--check')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('stale', result.stderr)
        self.assertEqual(before, [p.read_bytes() for p in pages])
        self.generate()
        self.assertIn('changedOpen', pages[0].read_text())
        pages[1].unlink()
        self.assertNotEqual(self.run_generator('--check').returncode, 0)
        self.assertFalse(pages[1].exists())

    def test_workspace_and_script_changes_update_contributor_index(self):
        pages = self.generate()
        before = pages[1].read_bytes()
        script = self.root / 'scripts/agent-build.sh'
        script.write_text(script.read_text() + '\n# fixture source change\n')
        self.assertNotEqual(self.run_generator('--check').returncode, 0)
        self.generate()
        self.assertNotEqual(before, pages[1].read_bytes())
        workspace = self.root / 'Cargo.toml'
        workspace.write_text(workspace.read_text().replace('members = [', 'members = [\n    "extra",', 1))
        extra = self.root / 'extra/Cargo.toml'
        extra.parent.mkdir()
        extra.write_text('[package]\nname = "fixture-added-member"\nversion = "77.88.99"\n')
        self.assertNotEqual(self.run_generator('--check').returncode, 0)
        self.generate()
        text = pages[1].read_text()
        self.assertNotEqual(before, pages[1].read_bytes())
        self.assertIn('fixture-added-member', text)
        self.assertNotIn('77.88.99', text)
        for verb in VERBS:
            self.assertIn(f'scripts/agent-{verb}.sh', text)
        extra.write_text(extra.read_text().replace('fixture-added-member', 'fixture-renamed-member'))
        self.assertNotEqual(self.run_generator('--check').returncode, 0)

    def test_invalid_or_missing_input_fails_without_partial_output(self):
        manifest = self.root / 'src/conformance/governed-operation-parity.json'
        data = json.loads(manifest.read_text())
        data['operations'][0]['rust']['locator'] = 'made-up-owner'
        manifest.write_text(json.dumps(data))
        self.assertNotEqual(self.run_generator('--write').returncode, 0)
        self.assertFalse((self.root / 'docs/generated').exists())
        shutil.copyfile(REPO / manifest.relative_to(self.root), manifest)
        (self.root / 'scripts/agent-test.sh').unlink()
        self.assertNotEqual(self.run_generator('--write').returncode, 0)
        self.assertFalse((self.root / 'docs/generated').exists())

    def test_missing_tomllib_has_actionable_runtime_diagnostic(self):
        code = """import builtins, runpy, sys
original = builtins.__import__
def absent(name, *args, **kwargs):
    if name == 'tomllib':
        raise ModuleNotFoundError('No module named tomllib')
    return original(name, *args, **kwargs)
builtins.__import__ = absent
sys.argv = [sys.argv[1], '--check']
runpy.run_path(sys.argv[0], run_name='__main__')
"""
        result = subprocess.run([sys.executable, '-c', code, str(GENERATOR)], text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Python 3.11', result.stderr)
        self.assertIn('.venv/bin/python', result.stderr)
        self.assertNotIn('Traceback', result.stderr)

    def test_public_navigation_and_shared_pipeline(self):
        builder = (REPO / 'scripts/release/build-pages-site.sh').read_text()
        self.assertIn('generate-public-docs.py', builder)
        self.assertIn('--check', builder)
        self.assertLess(builder.index('generate-public-docs.py'), builder.index('mkdocs build'))
        nav = (REPO / 'mkdocs.yml').read_text()
        for page in ('generated/sdk-operation-map.md', 'generated/contributor-map.md', 'contributing/index.md'):
            self.assertIn(page, nav)
        readme = (REPO / 'README.md').read_text()
        self.assertIn('docs/contributing/index.md', readme)
        self.assertIn('docs/generated/sdk-operation-map.md', readme)
        docs_workflow = (REPO / '.github/workflows/docs-pages.yml').read_text()
        for source in ('scripts/generate-public-docs.py', 'src/conformance/governed-operation-parity.json', 'Cargo.toml', 'src/rust/crates/**/Cargo.toml', 'scripts/agent-*.sh'):
            self.assertIn(f'- {source}', docs_workflow)
        for workflow, prepare, deploy in (
            ('docs-pages.yml', 'prepare-pages', 'deploy-pages'),
            ('jetson-tegra-cuda-evidence.yml', 'prepare-tegra-pages', 'deploy-tegra-pages'),
        ):
            source = (REPO / '.github/workflows' / workflow).read_text()
            # Job boundaries are column-two keys, not nested YAML properties.
            import re
            blocks = dict(re.findall(r'(?ms)^  ([a-z][a-z-]+):\n(.*?)(?=^  [a-z][a-z-]+:|\Z)', source))
            self.assertIn('check-pages-source.py', blocks[prepare])
            self.assertLess(blocks[prepare].index('check-pages-source.py'), blocks[prepare].index('build-pages-site.sh'))
            self.assertIn('fetch-depth: 0', blocks[prepare])
            self.assertIn('check-pages-source.py', blocks[deploy])
            self.assertLess(blocks[deploy].index('check-pages-source.py'), blocks[deploy].index('actions/deploy-pages@'))
            self.assertIn('--candidate "$GITHUB_SHA"', blocks[prepare])
            self.assertIn('--candidate "$GITHUB_SHA"', blocks[deploy])
            self.assertNotIn('contents:', blocks[deploy])
            self.assertIn('persist-credentials: false', blocks[prepare])


if __name__ == '__main__':
    unittest.main()
