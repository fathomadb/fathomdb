"""Report-cutoff recovery controls; authored tiny raw calls are never native evidence."""

import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import test_pair_selection
from g_pair import plan


class PairReportRecoveryTests(unittest.TestCase):
    def fixture(self, root):
        helper=test_pair_selection.PairSelectionTests()
        manifest,budget,selected,request=helper.fixture(root)
        original={'pid':7,'start':1,'root':str(root),'started_monotonic_ns':1000,'controller_pid':8}
        current={'pid':42,'start':2,'root':str(root),'started_monotonic_ns':500*10**9,'controller_pid':43}
        adoption=root/'selections/original.json';adoption.parent.mkdir()
        adoption.write_text(json.dumps({'schema':'slice50-pair-adoption/v1','selection':selected,
            'guardian_owner':original,'controller_source_sha':'original-controller',
            'native_source_sha':manifest['source_sha'],'compiler_inputs':manifest['compiler_inputs']}))
        receipt=dict(original,exit=124,stop='shared cleanup reserve',error=None,cleanup_uncertain=False,
            stopped_monotonic_ns=1000+101*10**9,accounting={'common_ns':81*10**9,
            'charged_ns':101*10**9,'recovered_cell_ns':0},boundary={'status':'quiescent',
            'populated_after':False,'removed_exact_empty_boundary':True,'guardian_pid':7,'guardian_start':1})
        guard=root/'guardians/original';guard.mkdir(parents=True);(guard/'guardian.json').write_text(json.dumps(receipt))
        rows=[]
        for cell in plan()['primary'][:2]:
            attempt=root/'results'/cell['id']/'attempt-0001';native=attempt/'native';native.mkdir(parents=True)
            for name,count in (('samples',cell['samples']),('warmups',cell['warmup'])):
                calls=[{'adapter':adapter,'route':'text','query_index':i%500,'sample_index':i,
                        'ok':True,'elapsed_ns':10,'ids':['logical'],'hits':[{'write_cursor':1,'score_bits':7}]}
                       for adapter in cell['adapters'] for i in range(count)]
                (native/(name+'.jsonl')).write_text(''.join(json.dumps(call)+'\n' for call in calls))
            native_result={'complete':True,'sample_count':2000,'warmup_count':1000,'first_call_count':0,
                'natural_records':10000,'vector_rows':0,'marker_records':0,
                'close_outcomes':[{'status':'ok'},{'status':'ok'}],
                'open_reports':[{'pid':9,'executable':str(root/'native'),'requested_provider':'none',
                                 'schema_version_after':37} for _ in cell['adapters']]}
            step={'output':str(native),'result':native_result,'complete':True,'exit':0,
                  'pid':9,'executable':str(root/'native')}
            result={'complete':True,'steps':[step],'qualification_issues':[],'cell':cell,
                    'freeze_uuid':manifest['uuid'],'scope':'primary','selected_build':manifest['build']['binaries']['ordinary']}
            (attempt/'result.json').write_text(json.dumps(result))
            row={'id':cell['id'],'cell':cell,'attempt':str(attempt),'status':'complete','complete':True,
                 'exit':0,'child_exit':0,'error':None,'cleanup_uncertain':False,'live_owned_pids':[],
                 'charged_ns':10*10**9,'guardian_owner':original,'selection':str(adoption)}
            (attempt/'attempt.json').write_text(json.dumps(row));rows.append(row)
        state=root/'results/campaign-state.json';state.write_text(json.dumps(rows))
        prior={'started_monotonic_ns':10,'controller_pid':11}
        prior_guard=root/'guardians/prior';prior_guard.mkdir()
        (prior_guard/'guardian.json').write_text(json.dumps(dict(prior,exit=0,cleanup_uncertain=False,
            boundary={'status':'quiescent','populated_after':False,'removed_exact_empty_boundary':True})))
        cached={'primary':[],'diagnostics':[],'calibration':[],'guardian_owner':prior,
                'active_members':[c['id'] for c in plan()['primary'][2:4]],
                'native_window_complete':True,'native_primary_complete':False,'native_calibration_complete':False}
        (root/'results/report.json').write_text(json.dumps(cached))
        selected=copy.deepcopy(selected);selected['primary_limits']['common_ns']=600*10**9
        selected['report_recovery']=str(adoption);request.write_text(json.dumps(selected))
        budget['s50_guardian']=current;(root/'g-budget.json').write_text(json.dumps(budget))
        return helper,manifest,selected,request,rows,receipt,current

    def assess(self, root, setup):
        from g_pair_freeze import validate
        from g_pair_runtime import control_entry
        helper,manifest,selected,request,rows,receipt,current=setup
        with helper.source():effective=validate(root,selection=request)
        effective['_selection_record']={'guardian_owner':current}
        with patch('g_pair_freeze.validate',return_value=effective), \
                patch('m_runtime.controller_admission',return_value=current), \
                patch('g_pair_report._statistics',return_value=[]), \
                patch('g_pair_runtime.control',side_effect=AssertionError('native control')), \
                patch('g_pair_runtime.execute_native',side_effect=AssertionError('native payload')), \
                patch('g_gpu.inspect_gpu',side_effect=AssertionError('GPU query')):
            self.assertTrue(control_entry('report',root,None,7,selection=request))
        return effective

    def finalize(self, root, owner, **changes):
        directory=root/'guardians/current';directory.mkdir(exist_ok=True)
        receipt=dict(owner,exit=0,cleanup_uncertain=False,boundary={'status':'quiescent',
            'populated_after':False,'removed_exact_empty_boundary':True});receipt.update(changes)
        (directory/'guardian.json').write_text(json.dumps(receipt))

    def test_exact_completed_window_gets_separate_assessment_not_retagged_guardian(self):
        from g_pair_report import published_report
        from g_pair_runtime import finish
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);setup=self.fixture(root);rows=setup[4]
            frozen={p:p.read_bytes() for p in (root/'manifest.json',root/'results/campaign-state.json',
                root/'guardians/original/guardian.json',root/'g-budget.json')}
            effective=self.assess(root,setup)
            pending=published_report(root)
            self.assertFalse(any(row['credited'] for row in pending['primary']))
            self.finalize(root,setup[6]);actual=published_report(root)
            self.assertTrue(actual['phase_qualified']);self.assertTrue(actual['window_complete'])
            self.assertEqual(actual['active_members'],[row['id'] for row in rows])
            self.assertEqual([row['id'] for row in actual['primary'] if row['credited']],[row['id'] for row in rows])
            record=actual['report_recoveries'][0]
            self.assertEqual(record['members'],[row['id'] for row in rows])
            self.assertEqual(record['original_guardian']['exit'],124)
            for path,data in frozen.items():self.assertEqual(path.read_bytes(),data)
            effective.pop('_report_recovery',None)
            with patch('g_pair_report._statistics',return_value=[]):finish(effective,rows)
            self.assertEqual(published_report(root)['report_recoveries'],actual['report_recoveries'])
            self.assertTrue(all(Path(row['attempt']).exists() for row in rows))

    def test_unsafe_original_execution_facts_refuse_before_history_or_cache_replacement(self):
        mutations=[('stop',lambda r,s:r.update(stop='primary phase cleanup reserve')),
            ('inflight',lambda r,s:r['accounting'].update(recovered_cell_ns=1)),
            ('below_common',lambda r,s:r['accounting'].update(common_ns=10*10**9)),
            ('uncertain',lambda r,s:r.update(cleanup_uncertain=True)),
            ('populated',lambda r,s:r['boundary'].update(populated_after=True)),
            ('owner',lambda r,s:s[0]['guardian_owner'].update(controller_pid=99)),
            ('incomplete',lambda r,s:s[0].update(complete=False)),
            ('native_error',lambda r,s:s[0].update(exit=1,error='native error')),
            ('pair_cap',lambda r,s:s[0].update(charged_ns=901*10**9))]
        for name,change in mutations:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);setup=self.fixture(root);receipt=setup[5];rows=copy.deepcopy(setup[4])
                change(receipt,rows)
                (root/'guardians/original/guardian.json').write_text(json.dumps(receipt))
                (root/'results/campaign-state.json').write_text(json.dumps(rows))
                cached=root/'results/report.json';before=cached.read_bytes()
                with self.assertRaisesRegex(ValueError,'recovery'):self.assess(root,setup)
                self.assertEqual(cached.read_bytes(),before)
                self.assertFalse(list(root.glob('results/report-history/*')))

    def test_missing_exact_window_or_native_raw_facts_cannot_recover(self):
        for missing in ('member','sample','native_complete'):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);setup=self.fixture(root)
                if missing=='member':
                    (root/'results/campaign-state.json').write_text(json.dumps(setup[4][:1]))
                else:
                    attempt=Path(setup[4][0]['attempt'])
                    if missing=='sample':(attempt/'native/samples.jsonl').write_text('')
                    else:
                        result=json.loads((attempt/'result.json').read_text());result['steps'][0]['complete']=False
                        (attempt/'result.json').write_text(json.dumps(result))
                with self.assertRaisesRegex(ValueError,'recovery'):self.assess(root,setup)

    def test_new_assessment_needs_healthy_guardian_and_unchanged_assessed_raw(self):
        from g_pair_report import published_report
        for failure in ('assessment_stop','raw_change'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);setup=self.fixture(root);self.assess(root,setup)
                self.finalize(root,setup[6],**({'exit':124,'stop':'shared cleanup reserve'} if failure=='assessment_stop' else {}))
                if failure=='raw_change':
                    with (Path(setup[4][0]['attempt'])/'native/samples.jsonl').open('a') as stream:stream.write('{}\n')
                actual=published_report(root)
                self.assertFalse(any(row['credited'] for row in actual['primary']))

    def test_recovery_selection_is_report_only_at_public_and_payload_seams(self):
        from g_pair import public_entry
        from g_pair_runtime import payload
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);setup=self.fixture(root);helper,manifest,selected,request,*_=setup
            with helper.source(),patch.dict(os.environ,AGENT_LONG='1',S30_TASK=str(root)), \
                    patch('m_guard.guard') as guard,self.assertRaisesRegex(ValueError,'report-only'):
                public_entry('run',root,selection=request)
            guard.assert_not_called()
            with helper.source(),patch('g_pair.worker_admission') as admission, \
                    patch('g_pair_runtime.execute_native') as native,self.assertRaisesRegex(ValueError,'report-only'):
                payload(root,root/'unused.json',root/'unused',10**30,selection=request)
            admission.assert_not_called();native.assert_not_called()

    def test_separate_credit_survives_public_run_finish_and_existing_retirement(self):
        import sqlite3
        import time
        from g_pair import public_entry
        from g_pair_runtime import control_entry
        from g_pair_report import published_report
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);setup=self.fixture(root);helper,manifest,selected,request,rows,_,current=setup
            for row in rows:
                store=Path(row['attempt'])/'subject/store.sqlite';store.parent.mkdir()
                connection=sqlite3.connect(store);connection.execute('CREATE TABLE data(value)');connection.close()
                row['store']=str(store)
            state=root/'results/campaign-state.json';state.write_text(json.dumps(rows));original=state.read_bytes()
            self.assess(root,setup);self.finalize(root,current)
            self.assertTrue(all(row['credited'] for row in published_report(root)['primary'][:2]))
            selected.pop('report_recovery');request.write_text(json.dumps(selected))
            owner={'pid':50,'start':3,'root':str(root),'started_monotonic_ns':time.monotonic_ns(),'controller_pid':51}
            def guard(command,**kwargs):
                budget=json.loads((root/'g-budget.json').read_text());budget['s50_guardian']=owner
                (root/'g-budget.json').write_text(json.dumps(budget))
                argv=command();adopted=Path(argv[argv.index('--selection')+1])
                with patch('m_runtime.controller_admission',return_value=owner), \
                        patch('g_pair_runtime.control',return_value=[]) as control:
                    self.assertFalse(control_entry('run',root,None,7,selection=adopted))
                control.assert_called_once()
                next_guard=root/'guardians/next';next_guard.mkdir()
                (next_guard/'guardian.json').write_text(json.dumps(dict(owner,exit=1,cleanup_uncertain=False,
                    boundary={'status':'quiescent','populated_after':False,'removed_exact_empty_boundary':True})))
                budget['s50_guardian']=None;(root/'g-budget.json').write_text(json.dumps(budget))
                return dict(owner,exit=1,cleanup_uncertain=False)
            with helper.source(),patch.dict(os.environ,AGENT_LONG='1',S30_TASK=str(root)), \
                    patch('m_guard.guard',side_effect=guard),patch('g_pair_report._statistics',return_value=[]):
                self.assertFalse(public_entry('run',root,selection=request))
            final=published_report(root)
            self.assertEqual(sum(row['credited'] for row in final['primary']),2)
            self.assertEqual(final['report_recoveries'][0]['original_guardian']['exit'],124)
            self.assertEqual(state.read_bytes(),original)
            self.assertTrue(all(not Path(row['store']).exists() for row in rows))

    def test_strict_clean_failed_assessment_never_confers_recovery_credit(self):
        from g_pair_report import published_report
        for outcome in ({'exit':1},{'exit':0,'child_exit':1}):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);setup=self.fixture(root);self.assess(root,setup)
                self.finalize(root,setup[6],**outcome)
                actual=published_report(root)
                self.assertTrue(actual['phase_qualified'], 'ordinary guardian policy stays unchanged')
                self.assertFalse(any(row['credited'] for row in actual['primary']))
                self.assertFalse(actual['window_complete'])


if __name__=='__main__':unittest.main()
