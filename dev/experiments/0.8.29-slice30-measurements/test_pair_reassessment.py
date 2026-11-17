"""Explicit CPU report adoption; source controls never dispatch native measurements."""

import contextlib
import fcntl
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from g_freeze import identify
import test_pair_selection
import test_pair_public


class PairReassessmentTests(unittest.TestCase):
    def test_cli_explicit_selection_recomputes_cached_report_only(self):
        import g_runner
        for admitted in (True, False):
            with self.subTest(admitted=admitted), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);(root/'results').mkdir()
                cached=root/'results/report.json';cached.write_text('{"old":true}')
                request=root/'request.json';request.write_text('{}')
                with patch.object(sys,'argv',['g_runner.py','pair-report',str(root),'--selection',str(request)]), \
                        patch('g_pair.public_entry',return_value=admitted) as entry, \
                        patch('g_pair_report.published_report',return_value={'current':True}) as published, \
                        patch('g_pair_runtime.control',side_effect=AssertionError('native dispatch')), \
                        contextlib.redirect_stdout(io.StringIO()) as output:
                    if admitted:
                        g_runner.main()
                        self.assertEqual(json.loads(output.getvalue()),{'current':True})
                        published.assert_called_once_with(root)
                    else:
                        with self.assertRaises(SystemExit) as error:g_runner.main()
                        self.assertEqual(error.exception.code,1)
                        published.assert_not_called()
                entry.assert_called_once_with('report',root,selection=request)
                self.assertEqual(cached.read_text(),'{"old":true}')

    def test_default_cli_remains_cached_without_admission(self):
        import g_runner
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);(root/'results').mkdir();(root/'results/report.json').write_text('{}')
            with patch.object(sys,'argv',['g_runner.py','pair-report',str(root)]), \
                    patch('g_pair.public_entry') as entry, \
                    patch('g_pair_report.published_report',return_value={'historical':True}), \
                    contextlib.redirect_stdout(io.StringIO()) as output:
                g_runner.main()
            entry.assert_not_called()
            self.assertEqual(json.loads(output.getvalue()),{'historical':True})

    def test_guarded_report_adopts_current_source_without_retirement_or_native(self):
        from g_pair import public_entry
        helper=test_pair_selection.PairSelectionTests()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,budget,selected,request=helper.fixture(root)
            original=(root/'manifest.json').read_bytes()
            owner={'pid':42,'start':1,'root':str(root),'started_monotonic_ns':1000}
            commands=[]
            def guard(command,**kwargs):
                self.assertEqual(kwargs['primary_limits'],selected['primary_limits'])
                self.assertEqual(kwargs['common_limit_ns'],selected['primary_limits']['common_ns'])
                with (root/'g-campaign.lock').open('a') as lock:
                    fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
                    current=dict(budget,s50_guardian=owner)
                    (root/'g-budget.json').write_text(json.dumps(current))
                    commands.append(command())
                    current.update(s50_guardian=None,used_ns=budget['used_ns']+91)
                    (root/'g-budget.json').write_text(json.dumps(current))
                return dict(owner,exit=0,cleanup_uncertain=False)
            with helper.source(),patch.dict(os.environ,AGENT_LONG='1',S30_TASK=str(root)), \
                    patch('m_guard.guard',side_effect=guard), \
                    patch('g_pair_runtime.retire_qualified') as retire, \
                    patch('g_pair_runtime.control',side_effect=AssertionError('native dispatch')):
                self.assertTrue(public_entry('report',root,selection=request))
            retire.assert_not_called()
            argv=commands[0]
            self.assertEqual(argv[argv.index('--operation')+1],'report')
            saved=json.loads(Path(argv[argv.index('--selection')+1]).read_text())
            self.assertEqual(saved['controller_source_sha'],'python-controller-revision')
            self.assertEqual(saved['native_source_sha'],'native-origin')
            self.assertEqual(saved['selection'],selected)
            self.assertEqual((root/'manifest.json').read_bytes(),original)
            current=json.loads((root/'g-budget.json').read_text())
            self.assertEqual(current['used_ns'],budget['used_ns']+91)
            self.assertEqual(current['cell_used_ns'],budget['cell_used_ns'])

    def test_invalid_report_selection_refuses_before_guard_or_history(self):
        from g_pair import public_entry
        helper=test_pair_selection.PairSelectionTests()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,budget,selected,request=helper.fixture(root)
            selected['manifest_uuid']='wrong-freeze';request.write_text(json.dumps(selected))
            with helper.source(),patch.dict(os.environ,AGENT_LONG='1',S30_TASK=str(root)), \
                    patch('m_guard.guard') as guard, self.assertRaisesRegex(ValueError,'campaign'):
                public_entry('report',root,selection=request)
            guard.assert_not_called()
            self.assertEqual(json.loads((root/'g-budget.json').read_text()),budget)
            self.assertFalse(list(root.glob('results/report-history/*')))

    def test_report_control_preserves_old_assessment_and_window_without_native(self):
        from g_pair_runtime import control_entry
        from g_pair import plan
        helper=test_pair_public.PairIncrementalRetirementTests()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,rows,report=helper.fixture(root)
            owner={'pid':42,'start':1,'root':str(root),'started_monotonic_ns':3,'controller_pid':4}
            prior_owner={'started_monotonic_ns':1,'controller_pid':2}
            old=dict(report,guardian_owner=prior_owner,active_members=[row['id'] for row in rows[2:]],
                native_primary_complete=False,native_calibration_complete=False,native_window_complete=False,
                original_full_stream_equal=False,controller_source_sha='old-controller')
            cached=root/'results/report.json';cached.write_text(json.dumps(old)+'\n');old_bytes=cached.read_bytes()
            helper.final_guardian(root,prior_owner,exit=1)
            manifest.pop('_active_members')
            manifest.update(plan=plan(),source_sha='native-origin',compiler_inputs='same-native-inputs',
                controller_source_sha='current-controller',_selection_record={'guardian_owner':owner})
            (root/'g-budget.json').write_text(json.dumps({'s50_guardian':owner}))
            state=(root/'results/campaign-state.json').read_bytes()
            with patch('g_pair_freeze.validate',return_value=manifest), \
                    patch('m_runtime.controller_admission',return_value=owner), \
                    patch('g_pair.report',return_value=report), \
                    patch('g_pair_runtime.control',side_effect=AssertionError('native dispatch')), \
                    patch('g_pair_freeze.prepare_inputs',side_effect=AssertionError('native preparation')), \
                    patch('g_gpu.inspect_gpu',side_effect=AssertionError('GPU query')):
                self.assertTrue(control_entry('report',root,None,7,selection=root/'adopted.json'))
            current=json.loads(cached.read_text())
            self.assertEqual(current['active_members'],old['active_members'])
            self.assertTrue(current['native_window_complete'])
            self.assertEqual(current['controller_source_sha'],'current-controller')
            assessment=current['reassessment']
            previous=Path(assessment['previous_report']['path'])
            self.assertEqual(previous.read_bytes(),old_bytes)
            self.assertEqual(assessment['previous_report'],identify(previous))
            self.assertEqual(assessment['previous_guardian']['exit'],1)
            self.assertEqual(assessment['native_source_sha'],'native-origin')
            self.assertEqual(assessment['compiler_inputs'],'same-native-inputs')
            self.assertEqual((root/'results/campaign-state.json').read_bytes(),state)
            self.assertTrue(all(Path(row['store']).exists() for row in rows))

    def test_reassessment_refuses_incomplete_pair_window_before_replacing_history(self):
        from g_pair_runtime import control_entry
        from g_pair import plan
        helper=test_pair_public.PairIncrementalRetirementTests()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,rows,report=helper.fixture(root)
            owner={'pid':42,'root':str(root),'started_monotonic_ns':3,'controller_pid':4}
            manifest.update(plan=plan(),source_sha='native-origin',compiler_inputs='same-native-inputs',
                controller_source_sha='current-controller',_selection_record={'guardian_owner':owner})
            (root/'g-budget.json').write_text(json.dumps({'s50_guardian':owner}))
            cached=root/'results/report.json'
            cached.write_text(json.dumps(dict(report,active_members=[rows[0]['id']])))
            original=cached.read_bytes()
            with patch('g_pair_freeze.validate',return_value=manifest), \
                    patch('m_runtime.controller_admission',return_value=owner), \
                    patch('g_pair.report',return_value=report), \
                    patch('g_pair_runtime.control',side_effect=AssertionError('native dispatch')), \
                    self.assertRaisesRegex(ValueError,'active members'):
                control_entry('report',root,None,7,selection=root/'adopted.json')
            self.assertEqual(cached.read_bytes(),original)
            self.assertFalse(list(root.glob('results/report-history/*')))


if __name__=='__main__':unittest.main()
