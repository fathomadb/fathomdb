"""Local CI composition: scoped default, full-workspace, installed-only, base ref, and timeout/cleanup regressions."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[4]


def _inspector_version():
    cargo = (ROOT / 'src/inspector/Cargo.toml').read_text(encoding='utf-8')
    in_pkg = False
    for line in cargo.splitlines():
        s = line.strip()
        if s.startswith('['):
            in_pkg = s == '[package]'
        elif in_pkg and s.startswith('version'):
            return s.split('=', 1)[1].strip().strip('"')
    raise AssertionError('missing [package].version in src/inspector/Cargo.toml')


VERSION = _inspector_version()


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
        (self.base / 'src/inspector/Cargo.toml').write_text(
            f'[package]\nname = "fathomdb-inspector"\nversion = "{VERSION}"\n',
            encoding='utf-8',
        )

    def _prepare_native_env(
        self,
        python_setup,
        *,
        diff_paths=(),
        base_resolvable=True,
        verify_exit=37,
        lint_exit=0,
        typecheck_exit=0,
        cargo_exit=0,
        node_exit=41,
    ):
        fixtures = self.base / 'fixtures'
        for fixture in ['person', 'vector', 'f06', 'study-training', 'study-unseen', 'lexical-deadline']:
            (fixtures / fixture / 'work').mkdir(parents=True, exist_ok=True)
            (fixtures / fixture / 'work/fixture.sqlite').write_text('not opened by this composition test')
        calls = self.base / 'calls.log'
        calls.unlink(missing_ok=True)
        scripts = self.base / 'scripts'
        scripts.mkdir(exist_ok=True)
        for name, code in {
            'agent-verify.sh': verify_exit,
            'agent-lint.sh': lint_exit,
            'agent-typecheck.sh': typecheck_exit,
        }.items():
            p = scripts / name
            p.write_text(f'#!/bin/sh\necho "{name}" >> "{calls}"\nexit {code}\n')
            p.chmod(0o755)
        venv = self.base / '.venv/bin'
        venv.mkdir(parents=True, exist_ok=True)
        if (venv / 'python').exists() or (venv / 'python').is_symlink():
            (venv / 'python').unlink()
        python_setup(venv / 'python')
        bins = self.base / 'bin'
        bins.mkdir(exist_ok=True)
        diff_output = '\\n'.join(diff_paths)
        git_script = f'''#!/bin/sh
echo "git $*" >> "{calls}"
case "$1" in
  rev-parse)
    if [ "$2" = "--verify" ]; then
      exit {0 if base_resolvable else 1}
    fi
    echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    exit 0
    ;;
  diff)
    printf "{diff_output}\\n"
    exit 0
    ;;
  status)
    exit 0
    ;;
esac
exit 0
'''
        cargo_script = f'''#!/bin/sh
echo "cargo $*" >> "{calls}"
exit {cargo_exit}
'''
        node_script = f'''#!/bin/sh
echo "node $*" >> "{calls}"
exit {node_exit}
'''
        # Deliberately poison docker to verify native local-ci.sh never requires or invokes Docker.
        for name, content in {
            'docker': '#!/bin/sh\necho "unexpected docker call" >&2\nexit 99\n',
            'git': git_script,
            'cargo': cargo_script,
            'node': node_script,
            'Xvfb': '#!/bin/sh\nexit 0\n',
            'flock': '#!/bin/sh\nexit 0\n',
            'readelf': '#!/bin/sh\nexit 0\n',
        }.items():
            (bins / name).write_text(content)
            (bins / name).chmod(0o755)
        browser = self.base / 'browser'
        browser.mkdir(exist_ok=True)
        (browser / 'chrome').write_text('test prerequisite only')
        (browser / 'chrome').chmod(0o755)
        modules = self.base / 'modules/playwright'
        modules.mkdir(parents=True, exist_ok=True)
        (modules / 'index.mjs').write_text('')
        env = dict(
            os.environ,
            PATH=str(bins) + ':' + os.environ['PATH'],
            INSPECTOR_CHROMIUM_DIRECTORY=str(browser),
            INSPECTOR_NODE_MODULES=str(modules.parent),
        )
        return fixtures, env, calls

    def test_missing_fixture_prerequisite_is_actionable_failure(self):
        result = subprocess.run(
            ['bash', str(self.script), str(self.base / 'candidate'), str(self.base / 'absent'), str(self.base)],
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('prerequisite:', result.stderr)
        self.assertIn('fixtures', result.stderr)
        self.assertFalse((self.base / 'candidate').exists())

    def test_default_scoped_runs_inspector_source_without_agent_verify_or_duplicate_unittest(self):
        tools_tests = self.base / 'src/inspector/tests/tools'
        tools_tests.mkdir(parents=True)
        unittest_log = self.base / 'unittest.log'
        (tools_tests / 'test_dummy.py').write_text(
            f'import unittest\nclass T(unittest.TestCase):\n    def test_ok(self):\n        open({str(unittest_log)!r}, "a").write("run\\n")\n',
            encoding='utf-8',
        )
        fixtures, env, calls = self._prepare_native_env(
            lambda p: p.symlink_to(sys.executable),
            diff_paths=('src/inspector/assets/app.js', 'docs/inspector/index.md'),
            node_exit=41,
        )
        result = subprocess.run(
            ['bash', str(self.script), str(self.base / 'candidate'), str(fixtures), str(self.base)],
            env=env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 41, result.stderr)
        log = calls.read_text(encoding='utf-8')
        self.assertIn('agent-lint.sh', log)
        self.assertIn('agent-typecheck.sh', log)
        self.assertIn('cargo test --locked --offline -p fathomdb-inspector --jobs 1 -- --test-threads=1', log)
        self.assertIn(
            'cargo test --locked --offline -p fathomdb-inspector --no-default-features --features cached-cpu --jobs 1 -- --test-threads=1',
            log,
        )
        self.assertNotIn('agent-verify.sh', log)
        self.assertEqual(unittest_log.read_text(encoding='utf-8').splitlines(), ['run'])

    def test_full_workspace_invokes_agent_verify_without_docker_and_preserves_exit(self):
        fixtures, env, calls = self._prepare_native_env(lambda p: p.symlink_to(sys.executable))
        result = subprocess.run(
            [
                'bash',
                str(self.script),
                '--full-workspace',
                str(self.base / 'candidate'),
                str(fixtures),
                str(self.base),
            ],
            env=env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 37, result.stderr)
        self.assertIn('stage=source', result.stderr)
        self.assertIn('agent-verify.sh', calls.read_text(encoding='utf-8'))
        self.assertNotIn('docker', result.stderr.lower())
        self.assertFalse((self.base / 'candidate').exists())

    def test_mutually_exclusive_and_unknown_flags_exit_2(self):
        fixtures, env, _ = self._prepare_native_env(lambda p: p.symlink_to(sys.executable))
        args = [str(self.base / 'candidate'), str(fixtures), str(self.base)]
        for flags in [
            ['--scoped', '--full-workspace'],
            ['--scoped', '--installed-only'],
            ['--full-workspace', '--installed-only'],
            ['--unknown-flag'],
            ['--base'],
        ]:
            result = subprocess.run(
                ['bash', str(self.script), *flags, *args],
                env=env,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 2, f'expected exit 2 for {flags}: {result.stderr}')

    def test_installed_only_requires_three_args_and_skips_source(self):
        fixtures, env, calls = self._prepare_native_env(lambda p: p.symlink_to(sys.executable))
        no_args = subprocess.run(
            ['bash', str(self.script), '--installed-only'],
            env=env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(no_args.returncode, 2)
        (self.base / 'src/inspector/tools/local-release.py').write_text(
            'import sys; sys.exit(43)\n',
            encoding='utf-8',
        )
        result = subprocess.run(
            [
                'bash',
                str(self.script),
                '--installed-only',
                str(self.base / 'candidate'),
                str(fixtures),
                str(self.base),
            ],
            env=env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 43, result.stderr)
        self.assertIn('stage=candidate', result.stderr)
        log = calls.read_text(encoding='utf-8') if calls.exists() else ''
        self.assertNotIn('agent-verify.sh', log)
        self.assertNotIn('agent-lint.sh', log)

    def test_scoped_custom_base_and_unresolvable_base_fail_closed(self):
        fixtures, env, calls = self._prepare_native_env(
            lambda p: p.symlink_to(sys.executable),
            diff_paths=('src/inspector/src/storage.rs',),
            base_resolvable=True,
            lint_exit=29,
        )
        ok = subprocess.run(
            [
                'bash',
                str(self.script),
                '--scoped',
                '--base',
                'custom-base-ref',
                str(self.base / 'candidate'),
                str(fixtures),
                str(self.base),
            ],
            env=env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(ok.returncode, 29, ok.stderr)
        self.assertIn('git diff --name-only custom-base-ref...HEAD', calls.read_text(encoding='utf-8'))

        fixtures2, env2, calls2 = self._prepare_native_env(
            lambda p: p.symlink_to(sys.executable),
            base_resolvable=False,
        )
        bad = subprocess.run(
            [
                'bash',
                str(self.script),
                '--scoped',
                '--base',
                'missing-ref',
                str(self.base / 'candidate2'),
                str(fixtures2),
                str(self.base),
            ],
            env=env2,
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(bad.returncode, 0)
        self.assertIn('missing-ref', bad.stderr)
        self.assertIn('--full-workspace', bad.stderr)
        log2 = calls2.read_text(encoding='utf-8')
        self.assertNotIn('agent-lint.sh', log2)

    def test_scoped_out_of_scope_diff_fails_closed_before_stages(self):
        fixtures, env, calls = self._prepare_native_env(
            lambda p: p.symlink_to(sys.executable),
            diff_paths=('src/inspector/assets/app.js', 'src/rust/crates/fathomdb-engine/src/lib.rs'),
        )
        result = subprocess.run(
            ['bash', str(self.script), str(self.base / 'candidate'), str(fixtures), str(self.base)],
            env=env,
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('src/rust/crates/fathomdb-engine/src/lib.rs', result.stderr)
        self.assertIn('--full-workspace', result.stderr)
        log = calls.read_text(encoding='utf-8')
        self.assertNotIn('agent-lint.sh', log)
        self.assertNotIn('agent-verify.sh', log)

    def test_optional_positional_args_default_to_temp_workspace_in_full_workspace(self):
        fixtures, env, calls = self._prepare_native_env(lambda p: p.symlink_to(sys.executable))
        temp_root = self.base / 'auto-tmp'
        temp_root.mkdir()
        env = dict(env, TMPDIR=str(temp_root), INSPECTOR_FIXTURE_DIR=str(fixtures))
        result = subprocess.run(
            ['bash', str(self.script), '--full-workspace'],
            env=env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 37, result.stderr)
        self.assertIn('agent-verify.sh', calls.read_text(encoding='utf-8'))
        self.assertEqual(list(temp_root.iterdir()), [], 'auto temp workspace must be cleaned on exit')

    def test_unusable_checkout_python_fails_before_source(self):
        def bad_python(path):
            path.write_text('#!/bin/sh\nexit 49\n')
            path.chmod(0o755)

        fixtures, env, _ = self._prepare_native_env(bad_python)
        result = subprocess.run(
            ['bash', str(self.script), str(self.base / 'candidate'), str(fixtures), str(self.base)],
            env=env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('prerequisite: checkout Python', result.stderr)
        self.assertFalse((self.base / 'candidate').exists())

    def test_run_installed_escalates_sigterm_ignoring_xvfb_to_sigkill(self):
        run_installed = ROOT / 'src/inspector/tools/run-installed.sh'
        pkg = self.base / 'pkg'
        pkg.mkdir()
        (pkg / 'fathomdb-inspector').write_text('#!/bin/sh\nexit 0\n')
        (pkg / 'fathomdb-inspector').chmod(0o755)
        fixtures = self.base / 'fixtures'
        fixtures.mkdir()
        target = self.base / 'target'
        fixture_bin = target / 'x86_64-unknown-linux-gnu/release/examples/fixture'
        fixture_bin.parent.mkdir(parents=True)
        fixture_bin.write_text('#!/bin/sh\nexit 0\n')
        fixture_bin.chmod(0o755)
        browser = self.base / 'browser'
        browser.mkdir()
        (browser / 'chrome').write_text('#!/bin/sh\necho "Chromium 156.0.8078.4"\n')
        (browser / 'chrome').chmod(0o755)
        modules = self.base / 'modules/playwright'
        modules.mkdir(parents=True)
        (modules / 'index.mjs').write_text('')
        cache = self.base / 'cache'
        embed = cache / 'fathomdb/embedders/0b2926f8a9b1'
        embed.mkdir(parents=True)
        for name in ['config.json', 'tokenizer.json', 'model.safetensors']:
            (embed / name).write_text('stub')
        xvfb_pid_file = self.base / 'xvfb.pid'
        bins = self.base / 'fakebin'
        bins.mkdir()
        (bins / 'Xvfb').write_text(
            f'#!/bin/sh\ntrap "" TERM\necho $$ > "{xvfb_pid_file}"\necho 99 >&3\nwhile true; do sleep 1; done\n',
            encoding='utf-8',
        )
        (bins / 'Xvfb').chmod(0o755)
        (bins / 'node').write_text('#!/bin/sh\nexit 0\n')
        (bins / 'node').chmod(0o755)
        (bins / 'python3').symlink_to(sys.executable)
        env = dict(
            os.environ,
            PATH=str(bins) + ':' + os.environ['PATH'],
            XDG_CACHE_HOME=str(cache),
            INSPECTOR_CHROMIUM_DIRECTORY=str(browser),
            INSPECTOR_NODE_MODULES=str(modules.parent),
        )
        started = time.monotonic()
        result = subprocess.run(
            ['bash', str(run_installed), str(pkg), str(fixtures), str(target), 'dummy.mjs'],
            env=env,
            capture_output=True,
            text=True,
            timeout=10,
        )
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertLess(elapsed, 6.0, f'Xvfb SIGKILL escalation took too long: {elapsed:.2f}s')
        self.assertGreaterEqual(elapsed, 1.5, 'Xvfb cleanup should wait ~2s before SIGKILL')
        pid = int(xvfb_pid_file.read_text(encoding='utf-8').strip())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)

    def test_installed_harness_avoids_stdout_pipe_deadlock_and_reaps_descendants(self):
        harness = (ROOT / 'src/inspector/tests/browser/installed-harness.mjs').read_text(encoding='utf-8')
        self.assertNotIn("stdio:['ignore','pipe','pipe']", harness)
        self.assertIn('killDescendantTree', harness)
        helpers = ROOT / 'src/inspector/tests/browser/helpers.mjs'
        script = f"""
import assert from 'node:assert/strict';
import {{ spawn }} from 'node:child_process';
import {{ readFile }} from 'node:fs/promises';
import {{ createInterface }} from 'node:readline';
import {{ setTimeout as delay }} from 'node:timers/promises';
import {{ killDescendantTree }} from {str(helpers)!r};

const child = spawn(
  {sys.executable!r},
  [
    '-c',
    'import signal, subprocess, sys, time; signal.signal(signal.SIGTERM, signal.SIG_IGN); '
    + 'grand = subprocess.Popen([sys.executable, "-c", "import signal, time; signal.signal(signal.SIGTERM, signal.SIG_IGN); time.sleep(60)"]); '
    + 'print(grand.pid, flush=True); time.sleep(60)',
  ],
  {{ stdio: ['ignore', 'pipe', 'ignore'] }},
);
const iter = createInterface({{ input: child.stdout }})[Symbol.asyncIterator]();
const grandPid = Number((await iter.next()).value.trim());
assert.ok(grandPid > 0);
const exited = new Promise((resolve) => child.once('exit', resolve));
const killed = await killDescendantTree(child.pid, 'SIGKILL');
assert.ok(killed.includes(grandPid), `expected grandchild ${{grandPid}} in killed tree ${{JSON.stringify(killed)}}`);
await Promise.race([exited, delay(3000).then(() => {{ throw new Error('parent not reaped'); }})]);
let grandLive = true;
for (let i = 0; i < 40; i++) {{
  try {{
    const stat = await readFile(`/proc/${{grandPid}}/stat`, 'utf8');
    const state = stat.slice(stat.lastIndexOf(')') + 2).split(' ')[0];
    if (state === 'Z') {{
      grandLive = false;
      break;
    }}
    await delay(25);
  }} catch {{
    grandLive = false;
    break;
  }}
}}
assert.equal(grandLive, false, 'grandchild descendant must no longer be live');
"""
        result = subprocess.run(
            ['node', '--input-type=module', '-e', script],
            capture_output=True,
            text=True,
            timeout=15,
        )
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()
