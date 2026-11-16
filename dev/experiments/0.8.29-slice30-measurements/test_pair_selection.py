"""Prospective selection controls; tiny retained files never qualify native measurements."""

import copy
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from g_pair import FEATURES, plan
from g_freeze import identify


class PairSelectionTests(unittest.TestCase):
    def fixture(self, root):
        native=root/'native';native.write_bytes(b'harmless-source-control')
        receipt=root/'build.json';receipt.write_text('{}')
        profile={'schema':'slice50-pair-budget/v1','status':'APPROVED','whole_cell_ns':900*10**9,
            'ledger':str(root/'g-budget.json'),'total_shared_ns':14400*10**9,
            'total_shared_retained_bytes':16*1024**3,'reserved_other_work_ns':0,
            'reserved_other_work_bytes':0,'other_work_cost_status':'HELD_UNAVAILABLE',
            'reservation_disposition':'finite selected prefix only; future costs unavailable'}
        limits={'pairs':1,'phase_ns':900*10**9,'pair_ns':800*10**9,
            'new_retained_bytes':2*1024**3,'common_ns':100*10**9,'common_bytes':256*1024**2}
        manifest={'schema':'slice50-paired-campaign/v1','scope':'primary','uuid':'original-freeze',
            'root':str(root),'allocation':str(root),'source_sha':'native-origin',
            'compiler_inputs':'same-native-inputs','features':FEATURES,'plan':plan(),
            'build':{'source_sha':'native-origin','binaries':{key:dict(identify(native),features=value)
                for key,value in FEATURES.items()}},'inputs':{},'model_files':{},'donors':{},'templates':{},
            'build_receipt':identify(receipt),'lock':identify(receipt),'environment':{},
            'profile':profile,'primary_limits':limits,'common_limit_ns':100*10**9,
            'retire_complete_copies':False,'g_donors':{},'model_cache':None,
            'requested_gpu':{'uuid':None,'ordinal':None}}
        (root/'manifest.json').write_text(json.dumps(manifest))
        budget={'schema':'slice30-gd-budget/v1','used_ns':123*10**9,'inflight':None,
            'cell_used_ns':{'prior-failed':17*10**9},'common_used_ns':{'D':11*10**9}}
        (root/'g-budget.json').write_text(json.dumps(budget))
        selected={'schema':'slice50-pair-selection/v1','root':str(root),'manifest_uuid':'original-freeze',
            'profile':dict(profile,total_shared_ns=48*3600*10**9,total_shared_retained_bytes=192*1024**3),
            'primary_limits':dict(limits,phase_ns=2000*10**9,pair_ns=1800*10**9),
            'retire_complete_copies':True}
        request=root/'request.json';request.write_text(json.dumps(selected))
        return manifest,budget,selected,request

    def source(self):
        return patch.multiple('g_pair_freeze',compiler_inputs=lambda:'same-native-inputs',
            git=lambda *args: '' if args[0]=='status' else 'python-controller-revision')

    def test_public_cli_accepts_explicit_existing_campaign_selection(self):
        result=subprocess.run([sys.executable,str(Path(__file__).with_name('g_runner.py')),
            'pair-run','--help'],capture_output=True,text=True)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertIn('--selection',result.stdout)

    def test_selected_larger_pair_budget_does_not_renew_old_attempted_pair_cap(self):
        from g_pair_runtime import deadline
        cells=plan()['primary'][:2]
        manifest={'scope':'primary','plan':plan(),'_original_pair_ns':800*10**9,
            'profile':{'whole_cell_ns':900*10**9,'total_shared_ns':48*3600*10**9,'reserved_other_work_ns':0},
            'primary_limits':{'phase_ns':2000*10**9,'pair_ns':1800*10**9}}
        budget={'used_ns':900*10**9,'cell_used_ns':{cells[0]['id']:800*10**9}}
        with self.assertRaisesRegex(ValueError,'allowance'):
            deadline(manifest,budget,{'started_monotonic_ns':1000*10**9},cells[1],1000*10**9)

    def test_explicit_native_equivalent_controller_continues_original_freeze(self):
        from g_pair_freeze import validate
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,budget,selected,request=self.fixture(root)
            original=(root/'manifest.json').read_bytes()
            with self.source():
                actual=validate(root,selection=request)
            self.assertEqual(actual['profile'],selected['profile'])
            self.assertEqual(actual['primary_limits'],selected['primary_limits'])
            self.assertTrue(actual['retire_complete_copies'])
            self.assertEqual(actual['source_sha'],'native-origin')
            self.assertEqual(actual['controller_source_sha'],'python-controller-revision')
            self.assertEqual(actual['build'],manifest['build'])
            self.assertEqual((root/'manifest.json').read_bytes(),original)
            self.assertEqual(json.loads((root/'g-budget.json').read_text()),budget)
            changed=copy.deepcopy(selected);changed['manifest_uuid']='another-freeze';request.write_text(json.dumps(changed))
            with self.source(),self.assertRaisesRegex(ValueError,'campaign'):
                validate(root,selection=request)

    def test_selection_validates_finite_total_phase_growth_and_unchanged_binding(self):
        from g_pair_freeze import validate
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,budget,selected,request=self.fixture(root)
            for field,value in [('total_shared_ns',0),('total_shared_ns',48*3600*10**9+1),
                    ('total_shared_retained_bytes',192*1024**3+1)]:
                changed=copy.deepcopy(selected);changed['profile'][field]=value;request.write_text(json.dumps(changed))
                with self.subTest(field=field,value=value),self.source(),self.assertRaises(ValueError):
                    validate(root,selection=request)
            for field,value in [('phase_ns',0),('phase_ns',49*3600*10**9),('pair_ns',1801*10**9),
                    ('new_retained_bytes',193*1024**3),('common_bytes',3*1024**3)]:
                changed=copy.deepcopy(selected);changed['primary_limits'][field]=value;request.write_text(json.dumps(changed))
                with self.subTest(field=field,value=value),self.source(),self.assertRaises(ValueError):
                    validate(root,selection=request)
            request.write_text(json.dumps(selected))
            with patch('g_pair_freeze.compiler_inputs',return_value='different-native-inputs'),self.source_git(),self.assertRaisesRegex(ValueError,'source|native'):
                validate(root,selection=request)

    def source_git(self):
        return patch('g_pair_freeze.git',side_effect=lambda *args:'' if args[0]=='status' else 'python-controller-revision')

    def test_guarded_adoption_keeps_old_outcomes_and_monotonic_shared_usage(self):
        from g_pair import public_entry
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,budget,selected,request=self.fixture(root)
            results=root/'results';results.mkdir()
            old=[{'id':plan()['primary'][0]['id'],'status':'complete','complete':True,'guardian_owner':{'controller_pid':7}},
                 {'id':'old-capped','status':'failed','complete':False,'reason':'absolute deadline'}]
            state=results/'campaign-state.json';state.write_text(json.dumps(old));old_bytes=state.read_bytes()
            owner={'pid':42,'start':1,'root':str(root),'started_monotonic_ns':1000}
            commands=[]
            def guard(command,**kwargs):
                self.assertEqual(kwargs['profile'],selected['profile'])
                self.assertEqual(kwargs['primary_limits'],selected['primary_limits'])
                with (root/'g-campaign.lock').open('a') as lock:
                    fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
                    current=json.loads((root/'g-budget.json').read_text());current['s50_guardian']=owner
                    (root/'g-budget.json').write_text(json.dumps(current))
                    commands.append(command())
                    current=json.loads((root/'g-budget.json').read_text());current['used_ns']+=91
                    current['s50_guardian']=None;(root/'g-budget.json').write_text(json.dumps(current))
                return dict(owner,exit=0,cleanup_uncertain=False)
            with self.source(),patch.dict(os.environ,AGENT_LONG='1',S30_TASK=str(root)),                    patch('m_guard.guard',side_effect=guard),patch('g_pair_runtime.retire_qualified') as retire:
                self.assertTrue(public_entry('run',root,selection=request))
            argv=commands[0];saved=Path(argv[argv.index('--selection')+1]);adopted=json.loads(saved.read_text())
            self.assertEqual(adopted['selection'],selected)
            self.assertEqual(adopted['guardian_owner'],owner)
            self.assertEqual(adopted['controller_source_sha'],'python-controller-revision')
            self.assertEqual(json.loads((root/'manifest.json').read_text()),manifest)
            self.assertEqual(state.read_bytes(),old_bytes)
            current=json.loads((root/'g-budget.json').read_text())
            self.assertEqual(current['used_ns'],budget['used_ns']+91)
            self.assertEqual(current['cell_used_ns'],budget['cell_used_ns'])
            self.assertEqual(current['common_used_ns'],budget['common_used_ns'])
            self.assertTrue(retire.call_args.kwargs['manifest']['retire_complete_copies'])

    def test_active_d_refuses_before_publishing_any_selection_or_changing_charges(self):
        from g_pair import public_entry
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,budget,selected,request=self.fixture(root)
            budget['d_guardian']={'pid':13,'status':'active'};(root/'g-budget.json').write_text(json.dumps(budget))
            with self.source(),patch.dict(os.environ,AGENT_LONG='1',S30_TASK=str(root)),                    self.assertRaisesRegex(RuntimeError,'ownership|inflight'):
                public_entry('run',root,selection=request)
            self.assertEqual(json.loads((root/'g-budget.json').read_text()),budget)
            self.assertFalse(list(root.glob('selections/*.json')))

    def test_success_credit_skips_old_members_and_failed_native_rows_stay_uncredited(self):
        from g_pair import required_prefix
        from g_pair_report import published_report
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,budget,selected,request=self.fixture(root)
            results=root/'results';results.mkdir()
            owner={'started_monotonic_ns':1,'controller_pid':2};guard=root/'guardians/old';guard.mkdir(parents=True)
            receipt=dict(owner,exit=0,cleanup_uncertain=False,boundary={'status':'quiescent',
                'populated_after':False,'removed_exact_empty_boundary':True})
            (guard/'guardian.json').write_text(json.dumps(receipt))
            rows=[dict(id=cell['id'],credited=True,guardian_owner=owner) for cell in plan()['primary'][:2]]
            rows += [dict(id=plan()['primary'][2]['id'],credited=False,status='failed')]
            cached={'guardian_owner':owner,'primary':rows,'diagnostics':[],'calibration':[],
                'native_primary_complete':False,'native_calibration_complete':False}
            (results/'report.json').write_text(json.dumps(cached))
            before=published_report(root)
            from g_pair_freeze import validate
            with self.source():effective=validate(root,selection=request)
            self.assertEqual(required_prefix(effective,root),plan()['primary'][2:4])
            self.assertEqual(published_report(root),before)
            self.assertFalse(before['primary'][2]['credited'])


    def test_selected_retirement_uses_final_credit_without_rewriting_frozen_option(self):
        from test_pair_public import PairIncrementalRetirementTests
        from g_pair_runtime import finish, retire_qualified
        helper=PairIncrementalRetirementTests()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);manifest,rows,report=helper.fixture(root)
            frozen=dict(manifest,retire_complete_copies=False)
            (root/'manifest.json').write_text(json.dumps(frozen));original=(root/'manifest.json').read_bytes()
            helper.final_guardian(root,rows[0]['guardian_owner'])
            with patch('g_pair.report',return_value=report):finish(manifest,rows[2:])
            budget=json.loads((root/'g-budget.json').read_text());budget['s50_guardian']=None
            (root/'g-budget.json').write_text(json.dumps(budget))
            retire_qualified(root,manifest=manifest)
            self.assertTrue(all(Path(row['store']).exists() for row in rows))
            helper.final_guardian(root,{'started_monotonic_ns':3,'controller_pid':4})
            retire_qualified(root,manifest=manifest)
            self.assertTrue(all(not Path(row['store']).exists() for row in rows))
            self.assertEqual((root/'manifest.json').read_bytes(),original)
            actual=json.loads((root/'g-budget.json').read_text())
            self.assertGreater(actual['used_ns'],0)
            self.assertEqual(actual['used_ns'],actual['common_used_ns']['S50'])


if __name__=='__main__':
    unittest.main()
