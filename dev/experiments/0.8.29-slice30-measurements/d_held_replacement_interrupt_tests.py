"""Actual C300k replacement controller death retains one-use history and charges."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

from d_runner import _interrupted_rows, run_guarded
from test_d_held_replacement import CELL, SPEC, SPENT, fixture
from d_driver import reviewed_quartet


class DHeldReplacementInterruptTests(unittest.TestCase):
    def test_owned_controller_death_retains_identityless_original_and_once_only_attempt2(self):
        outer=Path(os.environ['S30_TASK'])
        with tempfile.TemporaryDirectory(prefix='d-held-replace-',dir=outer/'tmp') as directory:
            task=Path(directory);root=task/'allocation/d';root.mkdir(parents=True)
            records,budget,review=fixture(root/'attempts')
            ledger=root.parent/'g-budget.json';state=root/'d-state.json';selected=root/'review.json'
            ledger.write_text(json.dumps(budget));state.write_text(json.dumps(records));selected.write_text(json.dumps(review))
            old=records[CELL];ready=task/'actual-selected-child'
            native='\n'.join(['import json,os,pathlib,signal,sys,time',
                'cell=json.loads(pathlib.Path(sys.argv[1]).read_text())',
                f'pathlib.Path({str(ready)!r}).write_text(cell["attempt_id"])',
                'os.kill(os.getppid(),signal.SIGTERM)','time.sleep(60)'])
            source=str(Path(__file__).parent.resolve())
            controller='\n'.join(['import json,os,pathlib,sys',f'sys.path.insert(0,{source!r})',
                'from d_driver import run_supervised_stage','from m_guard import write_budget',
                f'p=pathlib.Path({str(ledger)!r});state=pathlib.Path({str(state)!r})',
                f'review=json.loads(pathlib.Path({str(selected)!r}).read_text())',
                'budget=json.loads(p.read_text());records=json.loads(state.read_text())',
                "assert budget['d_guardian']['controller_pid']==os.getpid()",
                f'spec=json.loads(pathlib.Path({str(Path(__file__).with_name("d-cells.json").resolve())!r}).read_text())',
                'def publish():','    write_budget(state,records)','    write_budget(p,budget)',
                f'run_supervised_stage(spec,records,budget,pathlib.Path({str(root/"attempts")!r}),[sys.executable,"-c",{native!r}],initial=False,review=review,scientific=True,publish=publish)'])
            profile={'schema':'slice30-d-budget/v1','status':'APPROVED','ledger':str(ledger),
                'whole_cell_ns':4500*10**9,'total_shared_ns':172800*10**9,'total_shared_retained_bytes':192*1024**3,
                'reserved_other_work_ns':0,'reserved_other_work_bytes':0,'cell_limits_ns':{'300000':4500*10**9}}
            result=run_guarded([sys.executable,'-c',controller],allocation=root.parent,root=root,profile=profile,task=task)
            self.assertTrue(ready.exists(),'actual C300k replacement child never started before controller death')
            self.assertEqual(ready.read_text(),'attempt-0002')
            self.assertFalse(result['cleanup_uncertain'])
            self.assertTrue(result['boundary']['removed_exact_empty_boundary'])
            _interrupted_rows(root,root.parent,result)
            final=json.loads(ledger.read_text());after=json.loads(state.read_text())
            self.assertEqual(after[CELL]['status'],'interrupted')
            self.assertEqual(after[CELL]['attempt_id'],'attempt-0002')
            self.assertEqual(after[CELL]['prior_attempts'],[old])
            self.assertNotIn('attempt_id',after[CELL]['prior_attempts'][0])
            self.assertEqual(after[CELL]['elapsed_ns'],final['cell_used_ns'][CELL]-SPENT)
            self.assertEqual(final['used_ns']-budget['used_ns'],final['cell_used_ns'][CELL]-SPENT+final['common_used_ns']['D'])
            for key in records:
                if key!=CELL:
                    self.assertEqual(after[key],records[key])
            for key,token in budget['d_replacements'].items():
                self.assertEqual(final['d_replacements'][key],token)
            self.assertEqual((root/'attempts'/CELL/'attempt-0001/child.log').read_bytes(),b'original interrupted C300k raw bytes\n')
            final.update(shared_ns=budget['shared_ns'],storage_limit=budget['storage_limit'],retained_bytes=budget['retained_bytes'])
            before=final['used_ns']
            self.assertNotEqual(reviewed_quartet(SPEC,after,final,review)['status'],'admit')
            self.assertEqual(final['used_ns'],before)
            self.assertFalse((root/'attempts'/CELL/'attempt-0003').exists())


if __name__=='__main__':
    unittest.main()
