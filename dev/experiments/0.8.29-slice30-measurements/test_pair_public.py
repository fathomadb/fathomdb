"""Literal public pair inventory/report oracles; never natural qualification."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from g_pair import plan
from g_campaign import make_g_plan
from campaign import make_plan
from g_freeze import FEATURES


class PairInventoryTests(unittest.TestCase):
    def test_exact_primary_diagnostics_calibration_and_original_inventory(self):
        actual = plan()
        self.assertEqual(len(actual["primary"]), 136)
        self.assertEqual(len({row["origin_id"] for row in actual["primary"]}), 68)
        self.assertEqual(len(actual["diagnostics"]), 24)
        self.assertEqual(len({row["origin_id"] for row in actual["diagnostics"]}), 12)
        self.assertEqual(len(actual["calibration"]), 4)
        self.assertEqual(len({row["id"] for rows in actual.values() for row in rows}), 164)
        by_family = {name: sum(row["family"] == name for row in actual["primary"]) for name in ["P1", "P2", "G-R", "G-W"]}
        self.assertEqual(by_family, {"P1":8, "P2":24, "G-R":8, "G-W":96})
        for rows in actual.values():
            for row in rows:
                self.assertEqual((row["warmup"],row["samples"]), (500,1000))
                self.assertNotIn("embed", row["routes"])
                self.assertEqual(row["observe"], row["artifact"] == "diagnostic")
        for origin in {row["origin_id"] for row in actual["primary"]}:
            pair = [row for row in actual["primary"] if row["origin_id"] == origin]
            self.assertEqual([row["slice50_pair_mode"] for row in pair], ["eager","winner"] if origin.endswith("r1") else ["winner","eager"])
        for row in actual["calibration"]:
            self.assertEqual(row["origin_id"], "g-r-n10000-r0")
            self.assertEqual(row["n"], 10000)
            self.assertEqual(row["adapters"], ["engine","sdk"])
            self.assertEqual(row["routes"], ["text","hybrid"])
        tools = Path(__file__).parent
        self.assertEqual(len(make_g_plan(json.loads((tools/'g-cells.json').read_text()))),84)
        self.assertEqual(len(make_plan(json.loads((tools/'cells.json').read_text()),2)),16)
        self.assertEqual(FEATURES,{"ordinary":["embed-cuda"],"diagnostic":["embed-cuda","slice30-observer"]})

    def test_public_report_keeps_every_unrun_obligation_and_no_false_credit(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = subprocess.run([sys.executable, str(Path(__file__).with_name('g_runner.py')), 'pair-report', temporary],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            data=json.loads(result.stdout)
            self.assertFalse(data['complete'])
            self.assertEqual(len(data['primary']),136)
            self.assertEqual(len(data['diagnostics']),24)
            self.assertEqual(len(data['calibration']),4)
            self.assertTrue(all(row['status']=='not_run' for key in ['primary','diagnostics','calibration'] for row in data[key]))


class PairBuildBindingTests(unittest.TestCase):
    def test_harmless_builder_records_exact_private_features_and_stays_source_only(self):
        from unittest.mock import patch
        from g_pair import build_tools
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            built = root/'target/release/examples/slice30_measure'
            built.parent.mkdir(parents=True)
            calls = []
            def harmless(command, **kwargs):
                calls.append(command)
                subprocess.run([sys.executable,'-c',"import pathlib,sys;pathlib.Path(sys.argv[1]).write_bytes(b'source-control')",str(built)],check=True)
            with patch.dict('os.environ',{'CARGO_TARGET_DIR':str(root/'target')}):
                result = build_tools(root/'tools',_run=harmless)
            self.assertTrue(result['runner_only'])
            self.assertEqual(result['scope'],'slice50-pair')
            self.assertEqual(result['binaries']['ordinary']['features'],['embed-cuda','slice50-measurement'])
            self.assertEqual(result['binaries']['diagnostic']['features'],['embed-cuda','slice50-measurement','slice30-observer'])
            self.assertEqual(calls[0][calls[0].index('--features')+1],'embed-cuda,slice50-measurement')
            self.assertEqual(calls[1][calls[1].index('--features')+1],'embed-cuda,slice50-measurement,slice30-observer')
            self.assertTrue(all('test-hooks' not in row['features'] for row in result['binaries'].values()))

    def test_manifest_binds_current_feature_lists_and_donors_as_data_only(self):
        from g_pair import assemble_manifest
        build={'source_sha':'current', 'compiler_inputs':'current-inputs','binaries':{
            'ordinary':{'features':['embed-cuda','slice50-measurement'],'path':'owned/current'},
            'diagnostic':{'features':['embed-cuda','slice50-measurement','slice30-observer'],'path':'owned/current-diag'}}}
        donor={'G100000':{'origin':{'source_sha':'original'},'files':{'':{'path':'immutable-data'}}}}
        actual=assemble_manifest({'allocation':'original-shared','profile':{'status':'HELD'}},build,{'corpus':'registered'},donor)
        self.assertEqual(actual['features']['ordinary'],['embed-cuda','slice50-measurement'])
        self.assertEqual(actual['features']['diagnostic'],['embed-cuda','slice50-measurement','slice30-observer'])
        self.assertEqual(actual['source_sha'],'current')
        self.assertEqual(actual['donors'],donor)
        self.assertNotIn('reused',actual)
        self.assertEqual(len(actual['plan']['primary']),136)


class PairAdmissionTests(unittest.TestCase):
    def test_calibration_carries_actual_shared_and_per_id_usage(self):
        from g_pair import admit, CALIBRATION_ID
        limits={'phase_ns':100*10**9,'payload_ns':70*10**9,'new_retained_bytes':1000}
        profile={'total_shared_ns':1000*10**9,'total_shared_retained_bytes':10000,'reserved_other_work_ns':10*10**9,'reserved_other_work_bytes':100}
        budget={'used_ns':50*10**9,'calibration_usage':{CALIBRATION_ID:{'limits':limits,'wall_ns':20*10**9,'new_retained_bytes':100}}}
        result=admit(profile,budget,2000,limits)
        self.assertEqual(result['remaining_phase_ns'],80*10**9)
        self.assertEqual(result['remaining_retained_bytes'],900)
        budget['used_ns']=900*10**9
        with self.assertRaisesRegex(ValueError,'headroom'):
            admit(profile,budget,2000,limits)

    def test_exact_score_record_comparison_cannot_be_replaced_by_equal_public_ids(self):
        from g_pair import compare_results
        left={'samples':[{'ok':True,'ids':['h:equal'],'hits':[{'write_cursor':1,'score_bits':7,'source_id':'a'}]}], 'warmups':[], 'first_calls':[]}
        right=json.loads(json.dumps(left));right['samples'][0]['hits'][0]['write_cursor']=2
        self.assertFalse(compare_results(left,right))
        right=json.loads(json.dumps(left));right['samples'][0]['elapsed_ns']=999
        self.assertTrue(compare_results(left,right))

    def test_only_validated_quiescent_pair_retires_its_real_working_stores(self):
        import sqlite3
        from g_pair import retire_pair
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            donor=root/'immutable.sqlite'
            with sqlite3.connect(donor) as db: db.execute('CREATE TABLE data(value)');db.execute('INSERT INTO data VALUES(42)')
            before=donor.read_bytes()
            copies=[];outcomes=[]
            for mode in ['winner','eager']:
                working=root/'members'/mode/'subject/store.sqlite';working.parent.mkdir(parents=True)
                working.write_bytes(before);copies.append(working)
                outcomes.append({'status':'complete','complete':True,'cleanup_uncertain':False,'live_owned_pids':[],'store':str(working)})
            self.assertEqual(retire_pair(root/'members',outcomes,enabled=True,comparison=False,diagnostics_complete=True),[])
            self.assertTrue(all(path.exists() for path in copies))
            retired=retire_pair(root/'members',outcomes,enabled=True,comparison=True,diagnostics_complete=True)
            self.assertEqual(len(retired),2)
            self.assertTrue(all(not path.exists() for path in copies))
            self.assertEqual(donor.read_bytes(),before)


class PairPublicPipelineTests(unittest.TestCase):
    def test_full_calibration_dispatch_keeps_all_four_actual_child_invocations(self):
        from g_pair import drive_members, plan
        from g_driver import run_program_step
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary); calls=[]
            code="import json,os,pathlib,sys; c=json.loads(pathlib.Path(sys.argv[1]).read_text()); o=pathlib.Path(sys.argv[2]);o.mkdir();(o/'result.json').write_text(json.dumps({'complete':False,'runner_only':True,'mode':os.environ.get('FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT'),'cell':c}))"
            manifest={'scope':'calibration','plan':plan()}
            def actual_child(cell, index):
                import time
                output=root/f'actual{index}'
                step=run_program_step([sys.executable,'-c',code],cell,output,time.monotonic_ns()+5*10**9)
                calls.append(step)
                return {'id':cell['id'],'status':'failed','complete':False,'runner_only':True,'attempt':str(output)}
            rows=drive_members(manifest,root,actual_child)
            self.assertEqual(len(rows),4)
            self.assertEqual(len(calls),4)
            self.assertEqual([row['result']['mode'] for row in calls],[None,'1',None,'1'])
            self.assertEqual([row['result']['cell']['observe'] for row in calls],[False,False,True,True])
            self.assertEqual(len({row['output'] for row in calls}),4)
            self.assertTrue(all(not row['complete'] for row in rows))
            stored=json.loads((root/'results/campaign-state.json').read_text())
            self.assertEqual(len(stored),4)
            self.assertTrue(all(row['status']=='failed' for row in stored))

    def test_existing_guard_accounting_uses_the_pair_calibration_identity(self):
        import time
        from g_pair import CALIBRATION_ID
        from m_guard import Accounting
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            limits={'phase_ns':100*10**9,'payload_ns':70*10**9,'new_retained_bytes':1000}
            budget={'schema':'slice30-gd-budget/v1','used_ns':50*10**9,'inflight':None,
                'calibration_usage':{CALIBRATION_ID:{'limits':limits,'wall_ns':20*10**9,'new_retained_bytes':100}}}
            path=root/'g-budget.json';path.write_text(json.dumps(budget))
            began=time.monotonic_ns();accounting=Accounting(path,began,phase='S50')
            accounting.profile={'total_shared_ns':1000*10**9,'total_shared_retained_bytes':10000,'reserved_other_work_ns':0,'reserved_other_work_bytes':0}
            deadline=accounting.begin_calibration(limits,root/'stage',budget)
            self.assertEqual(deadline,began+80*10**9)
            self.assertEqual(budget['calibration_usage'][CALIBRATION_ID]['wall_ns'],20*10**9)

    def test_runner_only_and_missing_native_work_are_never_qualified(self):
        from g_pair import qualify_step, plan
        cell=plan()['calibration'][2]
        issues=qualify_step({'complete':True,'result':{'complete':True,'runner_only':True}},cell,{'gpu':{}})
        self.assertIn('runner_only_source_control',issues)
        self.assertIn('actual_native_reports_missing',issues)
        self.assertIn('diagnostic_work_unavailable',issues)


if __name__ == '__main__':
    unittest.main()


class PairPublicAdmissionTests(unittest.TestCase):
    def test_private_canonical_admission_refuses_missing_or_changed_members(self):
        from g_pair import canonical_members
        manifest={'schema':'slice50-paired-campaign/v1','scope':'calibration','features':{
            'ordinary':['embed-cuda','slice50-measurement'],
            'diagnostic':['embed-cuda','slice50-measurement','slice30-observer']},'plan':plan()}
        self.assertEqual(len(canonical_members(manifest)),4)
        changed=json.loads(json.dumps(manifest));changed['plan']['calibration'].pop()
        with self.assertRaisesRegex(ValueError,'canonical'):
            canonical_members(changed)
        changed=json.loads(json.dumps(manifest));changed['plan']['primary'][0]['samples']=1
        with self.assertRaisesRegex(ValueError,'canonical'):
            canonical_members(changed)

    def test_private_worker_requires_actual_parent_and_registered_exact_attempt(self):
        import os
        from unittest.mock import patch
        from g_pair import worker_admission
        from m_guard import process_start
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);attempt=root/'attempt';attempt.mkdir()
            manifest={'schema':'slice50-paired-campaign/v1','scope':'calibration','features':{
                'ordinary':['embed-cuda','slice50-measurement'],
                'diagnostic':['embed-cuda','slice50-measurement','slice30-observer']},
                'plan':plan(),'root':str(root),'allocation':str(root),'task':str(root),'uuid':'literal-freeze'}
            cell=manifest['plan']['calibration'][0]
            owner={'controller_pid':os.getppid(),'controller_start':process_start(os.getppid()),'root':str(root)}
            budget={'s50_guardian':owner,'inflight':{'cell':cell['id'],'attempt':str(attempt),'freeze_uuid':'literal-freeze'}}
            (root/'g-budget.json').write_text(json.dumps(budget))
            with patch('g_boundary.require_known_containment'):
                self.assertEqual(worker_admission(manifest,cell,attempt),owner)
                changed=dict(cell,samples=1)
                with self.assertRaisesRegex(RuntimeError,'canonical'):
                    worker_admission(manifest,changed,attempt)
                budget['inflight']['attempt']=str(root/'other');(root/'g-budget.json').write_text(json.dumps(budget))
                with self.assertRaisesRegex(RuntimeError,'canonical'):
                    worker_admission(manifest,cell,attempt)

    def test_public_calibration_rejects_old_g_profile_before_creating_any_root(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);profile=root/'profile.json';options=root/'options.json';out=root/'not-created'
            profile.write_text(json.dumps({'schema':'slice30-g-budget/v1','status':'APPROVED'}))
            options.write_text(json.dumps({'profile':str(profile),'allocation':str(root),
                'calibration':{'phase_ns':900*10**9,'payload_ns':800*10**9,'new_retained_bytes':2*1024**3}}))
            import os
            result=subprocess.run([sys.executable,str(Path(__file__).with_name('g_runner.py')),
                'pair-calibrate','--options',str(options),'--root',str(out)],
                env=dict(os.environ,AGENT_LONG='1'),capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('approved private Slice50 profile',result.stderr)
            self.assertFalse(out.exists())


class PairFreezeTests(unittest.TestCase):
    def test_source_only_builder_receipt_is_refused_before_input_access_or_publication(self):
        from g_pair import freeze
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);receipt=root/'build.json';out=root/'frozen'
            receipt.write_text(json.dumps({'schema':'slice50-pair-build/v1','runner_only':True}))
            with self.assertRaisesRegex(ValueError,'source-only build'):
                freeze({'build_receipt':str(receipt)},out,scope='calibration')
            self.assertFalse(out.exists())

    def test_primary_unknown_forecast_is_held_before_build_or_input_access(self):
        from g_pair import freeze
        with tempfile.TemporaryDirectory() as temporary:
            out=Path(temporary)/'frozen'
            with self.assertRaisesRegex(ValueError,'current paired forecasts unavailable'):
                freeze({},out,scope='primary')
            self.assertFalse(out.exists())


class PairNativePipelineTests(unittest.TestCase):
    def test_native_p_streams_are_loaded_for_exact_record_comparison(self):
        from g_pair import native_result
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            hit={'ok':True,'adapter':'sdk','query_index':0,'hits':[{'write_cursor':71,'score_bits':9}]}
            (root/'samples.jsonl').write_text(json.dumps(hit)+'\n')
            (root/'warmups.jsonl').write_text(json.dumps(hit)+'\n')
            (root/'marker-probes.jsonl').write_text(json.dumps({'marker_current':True,'write_cursor':72})+'\n')
            actual=native_result({'output':str(root),'result':{'sample_count':1,'open_report':{'pid':123}}})
            self.assertEqual(actual['samples'],[hit])
            self.assertEqual(actual['warmups'],[hit])
            self.assertEqual(actual['probes'][0]['write_cursor'],72)
            self.assertEqual(actual['open_reports'],[{'pid':123}])

    def test_pair_qualification_requires_closed_native_and_exact_call_records(self):
        from g_pair import qualify_step
        cell=plan()['primary'][0]
        incomplete={'complete':True,'result':{'complete':True,'sample_count':2000,'warmup_count':1000,
            'natural_records':10000,'vector_rows':0,'marker_records':0,'format':37}}
        issues=qualify_step(incomplete,cell,{})
        self.assertIn('native_close_facts_missing',issues)
        self.assertIn('exact_native_call_records_missing',issues)

    def test_public_report_refuses_equal_ids_with_different_actual_record_cursors(self):
        from g_pair import report
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);rows=[]
            for cell,cursor in zip(plan()['calibration'][:2],[71,72]):
                attempt=root/'results'/cell['id']/'attempt-0001';native=attempt/'native';native.mkdir(parents=True)
                call={'ok':True,'adapter':'sdk','route':'text','query_index':0,'sample_index':0,
                      'elapsed_ns':100,'ids':['h:same'],'hits':[{'write_cursor':cursor,'score_bits':9}]}
                (native/'samples.jsonl').write_text(json.dumps(call)+'\n')
                (attempt/'result.json').write_text(json.dumps({'complete':True,'steps':[{'output':str(native),'result':{}}]}))
                rows.append({'id':cell['id'],'cell':cell,'attempt':str(attempt),'status':'complete','complete':True,
                    'cleanup_uncertain':False,'live_owned_pids':[]})
            (root/'results/campaign-state.json').write_text(json.dumps(rows))
            actual=report(root)
            self.assertFalse(actual['calibration_complete'])
            pair=actual['comparisons']['calibration'][0]
            self.assertFalse(pair['equal'])
            self.assertEqual(pair['status'],'mismatch')
            self.assertTrue(all(row['status']=='not_run' for row in actual['primary']))


class PairRuntimeTests(unittest.TestCase):
    def test_calibration_deadline_keeps_prior_all_member_work_and_phase_freeze_time(self):
        from g_pair_runtime import deadline
        from g_pair import CALIBRATION_ID
        cell=plan()['calibration'][1]
        manifest={'scope':'calibration','plan':plan(),'profile':{'whole_cell_ns':900*10**9,
            'total_shared_ns':5000*10**9,'reserved_other_work_ns':0},
            'calibration':{'phase_ns':900*10**9,'payload_ns':800*10**9,'new_retained_bytes':2*1024**3}}
        budget={'used_ns':500*10**9,'calibration_usage':{CALIBRATION_ID:{'wall_ns':100*10**9}},
                'cell_used_ns':{plan()['calibration'][0]['id']:300*10**9,cell['id']:50*10**9}}
        self.assertEqual(deadline(manifest,budget,{'started_monotonic_ns':1000*10**9},cell,1100*10**9),1550*10**9)
        budget['cell_used_ns'][plan()['calibration'][0]['id']]=750*10**9
        with self.assertRaisesRegex(ValueError,'work allowance'):
            deadline(manifest,budget,{'started_monotonic_ns':1000*10**9},cell,1100*10**9)

    def test_existing_native_dispatch_consumes_validated_closed_copy_and_keeps_mode(self):
        from unittest.mock import patch
        from g_pair_runtime import execute_native
        from test_g_closed_copy import tiny_source
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);subject=tiny_source(root);cell=plan()['calibration'][0]
            calls=[]
            def current(command,config,output,end,observe=None):
                calls.append((command,config,output,end))
                self.assertTrue(Path(config['store']).is_file())
                self.assertNotEqual(config['store'],subject['files']['']['path'])
                self.assertEqual(config['slice50_pair_mode'],'winner')
                return {'complete':False,'result':{'runner_only':True}}
            manifest={'donors':{'G10000':subject},'inputs':{'corpus':{'path':'corpus'},'queries':{'path':'queries'}},
                      'build':{'binaries':{'ordinary':{'path':'current-private'}}}}
            import time
            end=time.monotonic_ns()+5*10**9
            with patch('g_driver.run_program_step',current):
                result=execute_native(manifest,cell,root/'member',end)
            self.assertEqual(len(calls),1)
            self.assertEqual(calls[0][0],['current-private'])
            self.assertEqual(calls[0][3],end)
            self.assertTrue(result['preparation']['validated'])
            self.assertFalse(result['complete'])


class PairCompletionTests(unittest.TestCase):
    def test_equal_partial_streams_cannot_credit_a_completed_pair(self):
        from g_pair import report
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);rows=[]
            for cell in plan()['calibration'][:2]:
                attempt=root/'results'/cell['id']/'attempt-0001';native=attempt/'native';native.mkdir(parents=True)
                (native/'samples.jsonl').write_text(json.dumps({'ok':True,'hits':[]})+'\n')
                (attempt/'result.json').write_text(json.dumps({'complete':False,'steps':[{'output':str(native),'result':{}}]}))
                rows.append({'id':cell['id'],'cell':cell,'attempt':str(attempt),'status':'complete','complete':True,
                    'cleanup_uncertain':False,'live_owned_pids':[]})
            (root/'results/campaign-state.json').write_text(json.dumps(rows))
            actual=report(root)
            pair=actual['comparisons']['calibration'][0]
            self.assertTrue(pair['equal'])
            self.assertFalse(pair['complete'])
            self.assertTrue(all(not row['credited'] for row in actual['calibration']))

    def test_unevidenced_primary_forecasts_are_unavailable(self):
        from g_pair_freeze import require_forecasts
        cells=plan()['primary']+plan()['diagnostics']
        options={'forecasts':{'status':'CURRENT_PAIRED_BOUNDED','common_ns':100*10**9,'common_bytes':100,
            'cells':{cell['id']:{'whole_ns':10**9,'new_retained_bytes':100,'observed_from':'/missing/current/paired'} for cell in cells}}}
        with self.assertRaisesRegex(ValueError,'current paired observation'):
            require_forecasts(options,cells)


class PairCommandTests(unittest.TestCase):
    def test_calibration_supports_the_selected_public_out_spelling(self):
        result=subprocess.run([sys.executable,str(Path(__file__).with_name('g_runner.py')),
            'pair-calibrate','--help'],capture_output=True,text=True)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertIn('--out',result.stdout)

    def test_calibration_cannot_be_resumed_as_an_unbounded_primary_run(self):
        from g_pair import public_entry
        import os
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            (root/'manifest.json').write_text(json.dumps({'scope':'calibration'}))
            with patch.dict(os.environ,{'AGENT_LONG':'1'}):
                with self.assertRaisesRegex(ValueError,'calibration executes only'):
                    public_entry('run',root)
