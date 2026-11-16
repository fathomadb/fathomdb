"""Run the real operator flow through its thin adapter with a harmless service child."""

from contextlib import ExitStack
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

import m_runner
import test_m_publication_write_cap as publication


class SupervisorFlowTests(unittest.TestCase):
    def setUp(self):
        self.fixture = publication.PublicationWriteCapTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root, self.base = self.fixture.root, self.fixture.base
        self.cell_id = self.fixture.cell['id']
        self.clock = 1000*10**9
        self.started = self.clock
        self.prior = 100*10**9
        budget = json.loads(self.fixture.ledger_path.read_text())
        budget['selected_cell_usage'][self.cell_id]['wall_ns'] = self.prior
        budget['used_ns'] = 1000*10**9
        self.fixture.ledger_path.write_text(json.dumps(budget))
        m_runner.report(self.root)
        self.options = self.base / 'operator-options.json'
        self.options.write_text(json.dumps({'allocation': str(self.base), '_controller_source_sha': 'literal-clean'}))
        self.native_started = self.base / 'service-started'
        self.events, self.child = [], None
        self.real_popen = subprocess.Popen
        self.real_log = m_runner.open_invocation_log
        self.real_invalidate = m_runner.invalidate_selected
        self.real_finalize = m_runner.finalize_invocation

    def invoke(self, *, fail_poll=False, overflow=False, exit_code=0, revocation_error=False, before_binding=False):
        samples = 0

        def resource(*_):
            nonlocal samples
            samples += 1
            self.clock += 2*10**9
            if fail_poll and samples == 2:
                raise RuntimeError('literal polling refusal')
            return {'memory_available': 100*1024**3, 'root_free': 100*1024**3,
                    'task_free': 1000*1024**3, 'owned': 0}

        def start(command, **kwargs):
            self.events.append('launch')
            if before_binding:
                (self.root/'manifest.json').unlink()
                for publication_path in (self.root/'reports').glob('report-*.json'):
                    publication_path.unlink()
            script = (f"import pathlib,sys,time;pathlib.Path({str(self.native_started)!r}).write_text('started');"
                      f"print('x'*100,flush=True);time.sleep(.05);sys.exit({exit_code})")
            self.child = self.real_popen([sys.executable, '-c', script], **kwargs)
            return self.child

        def control(command, **kwargs):
            if 'stop' in command:
                self.events.append('stop')
                self.clock += 10**9
                self.child.terminate()
            return subprocess.CompletedProcess(command, 0, stdout='')

        def observed(command, **kwargs):
            if command[0] == 'git':
                return 'literal-clean\n' if 'rev-parse' in command else ''
            self.events.append('strict-cleanup')
            self.clock += 10**9
            return 'ActiveState=inactive\nSubState=dead\nMainPID=0\nResult=success\nExecMainStatus=0\nControlGroup=\n'

        def log(root, kind):
            self.events.append('reserve-'+kind)
            handle = self.real_log(root, kind)
            if overflow and kind == 'service':
                handle.limit = 16
            return handle

        def invalidate(*args):
            self.events.append('invalidate')
            result = self.real_invalidate(*args)
            if revocation_error:
                raise RuntimeError("literal revocation failure")
            return result

        def finalize(*args):
            self.events.append('finalize')
            return self.real_finalize(*args)

        with ExitStack() as stack:
            stack.enter_context(patch('sys.argv', ['m_runner.py', 'supervise', self.cell_id, str(self.options), str(self.root),
                '--source', str(self.base), '--task', str(self.base), '--entry', str(self.base/'entry.sh')]))
            stack.enter_context(patch('m_runner.subprocess.Popen', side_effect=start))
            stack.enter_context(patch('m_runner.subprocess.run', side_effect=control))
            stack.enter_context(patch('m_runner.subprocess.check_output', side_effect=observed))
            stack.enter_context(patch('g_resource.resource_sample', side_effect=resource))
            stack.enter_context(patch('m_runner.time.monotonic_ns', side_effect=lambda: self.clock))
            stack.enter_context(patch('m_runner.open_invocation_log', side_effect=log))
            stack.enter_context(patch('m_runner.invalidate_selected', side_effect=invalidate))
            stack.enter_context(patch('m_runner.finalize_invocation', side_effect=finalize))
            try:
                return m_runner.main()
            except SystemExit as result:
                return result.code
            except (RuntimeError, FileExistsError):
                return 1
        return 0

    def assert_finalized_failed_invocation(self):
        self.assertIn('strict-cleanup', self.events)
        self.assertIn('invalidate', self.events)
        self.assertEqual(self.events.count('finalize'), 1)
        self.assertLess(self.events.index('strict-cleanup'), self.events.index('invalidate'))
        self.assertLess(self.events.index('invalidate'), self.events.index('finalize'))
        final = json.loads(self.fixture.ledger_path.read_text())
        usage = final['selected_cell_usage'][self.cell_id]
        self.assertEqual(usage['wall_ns'], self.prior+self.clock-self.started)
        latest = json.loads(sorted((self.root/'reports').glob('report-*.json'))[-1].read_text())
        self.assertFalse(latest['phase_qualified'])

    def test_capture_overflow_revokes_credit_then_accounts_the_whole_operator_invocation(self):
        self.assertNotEqual(self.invoke(overflow=True), 0)
        self.assertTrue(self.native_started.exists())
        self.assert_finalized_failed_invocation()

    def test_poll_refusal_still_performs_strict_cleanup_revocation_and_final_accounting(self):
        self.assertNotEqual(self.invoke(fail_poll=True), 0)
        self.assertIn('launch', self.events)
        self.assertIn('stop', self.events)
        self.assert_finalized_failed_invocation()

    def test_failed_service_body_is_revoked_and_accounted_after_strict_cleanup(self):
        self.assertNotEqual(self.invoke(exit_code=7), 0)
        self.assertEqual(self.child.returncode, 7)
        self.assert_finalized_failed_invocation()

    def test_revocation_error_cannot_bypass_final_cumulative_accounting(self):
        self.assertNotEqual(self.invoke(overflow=True, revocation_error=True), 0)
        self.assert_finalized_failed_invocation()

    def test_failed_body_before_input_binding_still_charges_the_operator_invocation(self):
        from test_m_calibration import LIMITS
        profile = json.loads((self.root/'manifest.json').read_text())['profile']
        profile.update(schema='slice30-m-budget/v1', status='APPROVED',
            ledger=str(self.fixture.ledger_path), whole_cell_ns=900*10**9,
            reserved_other_work_ns=0, reserved_other_work_bytes=0,
            other_work_cost_status='HELD_UNAVAILABLE', reservation_disposition='one unchanged M cell')
        profile_path = self.base/'profile.json'
        profile_path.write_text(json.dumps(profile))
        options = json.loads(self.options.read_text())
        options.update(profile=str(profile_path), cell_allowance=LIMITS)
        self.options.write_text(json.dumps(options))
        self.assertNotEqual(self.invoke(exit_code=7, before_binding=True), 0)
        self.assertEqual(self.events.count('finalize'), 1)
        usage = json.loads(self.fixture.ledger_path.read_text())['selected_cell_usage'][self.cell_id]
        self.assertEqual(usage['wall_ns'], self.prior+self.clock-self.started)
        self.assertEqual(usage['limits'], LIMITS)

    def test_samples_collision_is_detected_before_any_service_or_science_launch(self):
        collision = self.root.parent/(self.root.name+'-samples.jsonl')
        collision.write_text('immutable previous prefix\n')
        budget = self.fixture.ledger_path.read_bytes()
        self.assertNotEqual(self.invoke(), 0)
        self.assertNotIn('launch', self.events)
        self.assertFalse(self.native_started.exists())
        self.assertEqual(collision.read_text(), 'immutable previous prefix\n')
        self.assertEqual(self.fixture.ledger_path.read_bytes(), budget)


if __name__ == '__main__':
    unittest.main()
