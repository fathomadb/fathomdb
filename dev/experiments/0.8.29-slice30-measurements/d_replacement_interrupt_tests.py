"""Delegated harmless controller death retains consumed replacement identity/history."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

from d_runner import _interrupted_rows, run_guarded
from test_d_replacements import C, SPENT, T, fixture


class DReplacementInterruptTests(unittest.TestCase):
    def test_actual_controller_death_keeps_new_attempt_and_original_failure_without_double_charge(self):
        outer=Path(os.environ['S30_TASK'])
        with tempfile.TemporaryDirectory(prefix='d-replace-interrupt-',dir=outer/'tmp') as directory:
            task=Path(directory)
            root=task/'allocation'/'d'
            root.mkdir(parents=True)
            records,budget,review=fixture(root/'attempts')
            ledger=root.parent/'g-budget.json'
            state=root/'d-state.json'
            selected=root/'review.json'
            ledger.write_text(json.dumps(budget))
            state.write_text(json.dumps(records))
            selected.write_text(json.dumps(review))
            old=records[T]
            ready=task/'actual-replacement-child'
            native='\n'.join([
                'import json,os,pathlib,signal,sys,time',
                'cell=json.loads(pathlib.Path(sys.argv[1]).read_text())',
                f'pathlib.Path({str(ready)!r}).write_text(cell["attempt_id"])',
                'os.kill(os.getppid(),signal.SIGTERM)',
                'time.sleep(60)',
            ])
            source=str(Path(__file__).parent.resolve())
            controller='\n'.join([
                'import json,os,pathlib,sys',f'sys.path.insert(0,{source!r})',
                'from d_driver import run_supervised_stage', 'from m_guard import write_budget',
                f'p=pathlib.Path({str(ledger)!r}); state=pathlib.Path({str(state)!r})',
                f'review=json.loads(pathlib.Path({str(selected)!r}).read_text())',
                'budget=json.loads(p.read_text()); records=json.loads(state.read_text())',
                "assert budget['d_guardian']['controller_pid']==os.getpid()",
                f'spec=json.loads(pathlib.Path({str(Path(__file__).with_name("d-cells.json").resolve())!r}).read_text())',
                'def publish():',
                '    write_budget(state,records)',
                '    write_budget(p,budget)',
                f'run_supervised_stage(spec,records,budget,pathlib.Path({str(root/"attempts")!r}),[sys.executable,"-c",{native!r}],initial=False,review=review,scientific=True,publish=publish)',
            ])
            profile={'schema':'slice30-d-budget/v1','status':'APPROVED','ledger':str(ledger),
                'whole_cell_ns':1500*10**9,'total_shared_ns':3600*10**9,'total_shared_retained_bytes':16*1024**3,
                'reserved_other_work_ns':0,'reserved_other_work_bytes':0}
            result=run_guarded([sys.executable,'-c',controller],allocation=root.parent,
                               root=root,profile=profile,task=task)
            self.assertTrue(ready.exists(),'interruption control never reached the actual selected child')
            self.assertEqual(ready.read_text(),'attempt-0002')
            self.assertFalse(result['cleanup_uncertain'])
            self.assertTrue(result['boundary']['removed_exact_empty_boundary'])
            _interrupted_rows(root,root.parent,result)
            final=json.loads(ledger.read_text())
            after=json.loads(state.read_text())
            self.assertEqual(after[T]['attempt_id'],'attempt-0002',
                             'controller interruption lost the durably consumed replacement identity')
            self.assertEqual(after[T]['status'],'interrupted')
            self.assertEqual(after[T]['prior_attempts'],[old])
            self.assertEqual(after[T]['elapsed_ns'],final['cell_used_ns'][T]-SPENT[T])
            self.assertEqual(after[C],records[C])
            self.assertFalse((root/'attempts'/C/'attempt-0002').exists())
            self.assertEqual(final['used_ns']-budget['used_ns'],
                             final['cell_used_ns'][T]-SPENT[T]+final['common_used_ns']['D'])
            self.assertEqual(final['cell_used_ns'][C],SPENT[C])
            self.assertEqual((root/'attempts'/T/'attempt-0001'/'child.log').read_bytes(),
                             b'original unqualified raw bytes\n')


if __name__ == '__main__':
    unittest.main()
