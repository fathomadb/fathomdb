"""A live T bound refusal must stop C before replacement consumption/preparation."""

import copy
import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from d_attempt import run_owned_cell
from d_driver import reviewed_quartet, run_accounted_stage
from test_d_replacements import C, SPEC, SPENT, T, fixture


class DReplacementRefusalTests(unittest.TestCase):
    def test_live_t_cap_refusal_preserves_both_old_attempts_and_never_starts_c(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)/'attempts'
            records,budget,review=fixture(root)
            old=copy.deepcopy(records)
            review['bounds']['C']['time_ns']=200*10**9
            budget['shared_ns']=budget['used_ns']+500*10**9
            decision=reviewed_quartet(SPEC,records,budget,review)
            self.assertEqual(decision['status'],'admit')
            marker=Path(directory)/'forbidden-C-child'
            code='\n'.join([
                'import json,pathlib,sys',
                f'pathlib.Path({str(marker)!r}).write_text("started after T refusal")',
                '(pathlib.Path(sys.argv[2])/"result.json").write_text(json.dumps({"complete":False,"error":"literal C failure"}))',
            ])
            base=time.monotonic_ns()
            calls=[0]
            def clock():
                calls[0]+=1
                return base if calls[0] == 1 else base+250*10**9
            def execute(cell,deadline):
                return run_owned_cell(cell,root,[sys.executable,'-c',code],deadline)
            selected=dict(SPEC,cells=[cell for cell in SPEC['cells'] if cell['id'] in decision['cells']])
            rows=run_accounted_stage(selected,records,budget,execute,initial=False,clock=clock,
                fail_fast=True,replacements=decision['replacement_selection'],
                replacement_bounds=decision['predictions'],attempt_root=root)
            self.assertFalse(marker.exists(),'T cap refusal still allowed the C replacement child')
            by_id={row['id']:row for row in rows}
            self.assertEqual(by_id[T]['status'],'not_run_cap_feasibility')
            self.assertEqual(by_id[C]['status'],'not_run_failed_prefix')
            self.assertEqual(records,old)
            self.assertEqual(budget['cell_used_ns'][T],SPENT[T])
            self.assertEqual(budget['cell_used_ns'][C],SPENT[C])
            self.assertEqual(budget.get('d_replacements',{}),{})
            self.assertEqual(budget['common_used_ns']['D'],250*10**9)
            for key in (T,C):
                self.assertFalse((root/key/'attempt-0002').exists())
                self.assertFalse((root/key/'attempt-0003').exists())
                self.assertEqual((root/key/'attempt-0001'/'child.log').read_bytes(),
                                 b'original unqualified raw bytes\n')


if __name__ == '__main__':
    unittest.main()
