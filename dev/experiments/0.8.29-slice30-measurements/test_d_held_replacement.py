"""One C300k continuation preserves old TERM evidence and separates both limits."""

import copy
import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from d_attempt import run_owned_cell
from d_driver import reviewed_quartet, run_accounted_stage, run_supervised_stage
from test_d_controller import literal_records

SPEC = json.loads(Path(__file__).with_name('d-cells.json').read_text())
CELL = 'd-c-held-n300000-r1'
CURRENT = ('d-c-held-n300000-r0', 'd-t-held-n300000-r0', 'd-t-held-n300000-r1')
SPENT = 2552606408977


def fixture(root):
    """Literal source-control history; harmless children supply no scientific credit."""
    records = literal_records(10000, 100000, seconds=10, peak_bytes=8*1024**2)
    for cell in SPEC['cells']:
        if cell['id'] in CURRENT:
            records[cell['id']] = dict(cell, status='complete', elapsed_ns=10*10**9,
                peak_bytes=8*1024**2, error_count=0, physical_count=300000, closed=True,
                observations_complete=True, dropped_records=0, actual_overlap=None)
    for key,row in records.items():
        row.update(attempt_id='attempt-0001',attempt=str(root/key/'attempt-0001'))
    original=root/CELL/'attempt-0001'
    original.mkdir(parents=True)
    (original/'child.log').write_bytes(b'original interrupted C300k raw bytes\n')
    cell=next(cell for cell in SPEC['cells'] if cell['id']==CELL)
    (original/'cell.json').write_text(json.dumps(dict(cell,cell_ns=4500*10**9,attempt_id='attempt-0001')))
    records[CELL]={'id':CELL,'status':'interrupted',
        'error':'owned controller stopped before publishing its attempted record',
        'elapsed_ns':SPENT,'peak_bytes':None,'native':None,'cleanup_uncertain':False}
    cells={key:row['elapsed_ns'] for key,row in records.items()}
    budget={'schema':'slice30-gd-budget/v1','used_ns':41543944405554,'shared_ns':172800*10**9-60*10**9,
        'retained_bytes':90060746884,'storage_limit':192*1024**3,'cell_used_ns':cells,
        'common_used_ns':{'G':100*10**9},'inflight':None,'cell_limits_ns':{'300000':4500*10**9},
        'd_replacements':{'d-t-overlap-n100000-r1':{'historical':'preserved T'},
                          'd-c-overlap-n100000-r1':{'historical':'preserved C'}}}
    review={'verdict':'ACCEPT','reviewer':'literal reviewed C300k continuation','subject':'one interrupted C-r1 only',
        'n':300000,'mode':'held','checkpoint':{key:budget[key] for key in ('used_ns','retained_bytes')},
        'prefix_attempts':{key:row['attempt_id'] for key,row in records.items() if key not in (*CURRENT,CELL)},
        'qualified_current_attempts':{key:{field:records[key][field] for field in ('attempt_id','attempt')}
                                     for key in CURRENT},
        'replacements':{CELL:{'record':copy.deepcopy(records[CELL]),'charged_ns':SPENT,
            'old_attempt':{'attempt_id':'attempt-0001','attempt':str(original)},
            'next_attempt_id':'attempt-0002','cumulative_limit_ns':7500*10**9}},
        'bounds':{'C':{'time_ns':4320484082400,'active_bytes':4745905920},
                  'T':{'time_ns':4311382973460,'active_bytes':3964789260}}}
    return records,budget,review


class DHeldReplacementTests(unittest.TestCase):
    def test_only_pending_c_is_funded_with_separate_fresh_and_cumulative_allowances(self):
        with tempfile.TemporaryDirectory() as directory:
            records,budget,review=fixture(Path(directory))
            old=copy.deepcopy((records,budget))
            decision=reviewed_quartet(SPEC,records,budget,review)
            self.assertEqual(decision['status'],'admit','reviewed C300k continuation still refuses')
            self.assertEqual(decision['pending_cells'],[CELL])
            self.assertEqual(decision['remaining_ns'],{CELL:4500000000000})
            self.assertEqual(decision['cumulative_limits_ns'],{CELL:7500000000000})
            self.assertEqual(decision['planned_new_time_ns'],4320484082400)
            self.assertEqual(decision['planned_new_bytes'],4745905920)
            self.assertEqual(7500000000000-SPENT,4947393591023)
            self.assertEqual((records,budget),old)

    def test_real_failed_child_keeps_identityless_original_and_never_gets_attempt3(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)/'attempts';records,budget,review=fixture(root)
            old=copy.deepcopy(records);tokens=copy.deepcopy(budget['d_replacements'])
            marker=Path(directory)/'children'
            code='\n'.join(['import json,pathlib,sys',
                f'with pathlib.Path({str(marker)!r}).open("a") as out: out.write(json.loads(pathlib.Path(sys.argv[1]).read_text())["id"]+"\\n")',
                '(pathlib.Path(sys.argv[2])/"result.json").write_text(json.dumps({"complete":False,"error":"literal C failure"}))'])
            before=budget['used_ns']
            rows=run_supervised_stage(SPEC,records,budget,root,[sys.executable,'-c',code],
                                     initial=False,review=review,scientific=True)
            self.assertTrue(marker.exists(),'selected C300k fresh child never actually ran')
            self.assertEqual(marker.read_text().splitlines(),[CELL])
            self.assertEqual(len(rows),24)
            self.assertEqual(records[CELL]['attempt_id'],'attempt-0002')
            self.assertEqual(records[CELL]['status'],'failed')
            self.assertEqual(records[CELL]['prior_attempts'],[old[CELL]])
            self.assertEqual(records[CELL]['replacement_of'],dict(review['replacements'][CELL]['old_attempt'],
                status='interrupted',charged_ns=SPENT))
            self.assertNotIn('attempt_id',records[CELL]['prior_attempts'][0])
            for key in old:
                if key != CELL:
                    self.assertEqual(records[key],old[key])
            for key,value in tokens.items():
                self.assertEqual(budget['d_replacements'][key],value)
            self.assertEqual((root/CELL/'attempt-0001/child.log').read_bytes(),b'original interrupted C300k raw bytes\n')
            config=json.loads((root/CELL/'attempt-0002/cell.json').read_text())
            self.assertEqual(config['cell_ns'],4500000000000)
            self.assertGreater(config['remaining_cell_ms'],4493000)
            self.assertLessEqual(config['remaining_cell_ms'],4494000)
            self.assertEqual(budget['used_ns']-before,budget['cell_used_ns'][CELL]-SPENT+budget['common_used_ns']['D'])
            snapshot=copy.deepcopy((records,budget))
            run_supervised_stage(SPEC,records,budget,root,[sys.executable,'-c',code],
                                 initial=False,review=review,scientific=True)
            self.assertEqual((records,budget),snapshot)
            self.assertEqual(marker.read_text().splitlines(),[CELL])
            self.assertFalse((root/CELL/'attempt-0003').exists())

    def test_literal_history_identity_caps_and_other_cell_refusals_prepare_nothing(self):
        for change in ('old-row','charge','pin','prefix','raw-path','raw-config','missing-raw',
                       'cumulative-too-large','cumulative-too-small','fresh-too-small','other-cell'):
            with self.subTest(change=change),tempfile.TemporaryDirectory() as directory:
                root=Path(directory)/'attempts';records,budget,review=fixture(root)
                selected=review['replacements'][CELL]
                if change=='old-row': selected['record']['error']='changed'
                elif change=='charge': selected['charged_ns']+=1
                elif change=='pin': review['qualified_current_attempts'][CURRENT[0]]['attempt_id']='different'
                elif change=='prefix': review['prefix_attempts']['d-c-held-n10000-r0']='different'
                elif change=='raw-path': selected['old_attempt']['attempt']=str(Path(directory)/'foreign')
                elif change=='raw-config': (root/CELL/'attempt-0001/cell.json').write_text('{}')
                elif change=='missing-raw': (root/CELL/'attempt-0001/cell.json').unlink()
                elif change=='cumulative-too-large': selected['cumulative_limit_ns']=7500000000001
                elif change=='cumulative-too-small': selected['cumulative_limit_ns']=4500000000000
                elif change=='fresh-too-small': budget['cell_limits_ns']['300000']=4400000000000
                else: review['replacements']['d-t-held-n300000-r1']=review['replacements'].pop(CELL)
                before=copy.deepcopy((records,budget))
                decision=reviewed_quartet(SPEC,records,budget,review)
                self.assertNotEqual(decision['status'],'admit','exact C300k selection or original evidence was ignored')
                self.assertEqual((records,budget),before)
                self.assertFalse((root/CELL/'attempt-0002').exists())

    def test_live_shared_refusal_preserves_old_row_and_does_not_consume_attempt(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)/'attempts';records,budget,review=fixture(root)
            budget['shared_ns']=budget['used_ns']+4321*10**9
            decision=reviewed_quartet(SPEC,records,budget,review)
            self.assertEqual(decision['status'],'admit')
            old=copy.deepcopy(records);tokens=copy.deepcopy(budget['d_replacements'])
            marker=Path(directory)/'forbidden';base=time.monotonic_ns();calls=[0]
            def clock():
                calls[0]+=1
                return base if calls[0]==1 else base+10*10**9
            def execute(cell,deadline):
                return run_owned_cell(cell,root,[sys.executable,'-c',f'from pathlib import Path;Path({str(marker)!r}).touch()'],deadline)
            selected=dict(SPEC,cells=[dict(cell,cell_ns=4500000000000) for cell in SPEC['cells']
                                     if cell['id'] in decision['cells']])
            rows=run_accounted_stage(selected,records,budget,execute,initial=False,clock=clock,fail_fast=True,
                replacements=decision['replacement_selection'],replacement_bounds=decision['predictions'],
                replacement_cumulative_limits=decision['cumulative_limits_ns'],attempt_root=root)
            self.assertFalse(marker.exists())
            self.assertEqual(records,old)
            self.assertEqual(budget['d_replacements'],tokens)
            self.assertEqual(budget['cell_used_ns'][CELL],SPENT)
            self.assertEqual({r['id']:r for r in rows}[CELL]['status'],'not_run_cap_feasibility')
            self.assertFalse((root/CELL/'attempt-0002').exists())


if __name__=='__main__':
    unittest.main()
