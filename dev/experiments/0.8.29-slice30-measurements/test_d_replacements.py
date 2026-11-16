"""One explicit T/C replacement pair; old attempts and cumulative allowances survive."""

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

from d_driver import reviewed_quartet, run_supervised_stage
from test_d_controller import literal_records


SPEC = json.loads(Path(__file__).with_name('d-cells.json').read_text())
T, C = 'd-t-overlap-n100000-r1', 'd-c-overlap-n100000-r1'
SPENT = {T: 19391795054, C: 42471828234}


def fixture(root):
    """Hand-stated qualification/cost data only; no native or database evidence."""
    records = literal_records(10000, 100000, seconds=10, peak_bytes=8*1024**2)
    for key, row in records.items():
        row.update(attempt_id='attempt-0001', attempt=str(root/key/'attempt-0001'))
    for key, status in ((T, 'failed'), (C, 'interrupted')):
        path = root/key/'attempt-0001'
        path.mkdir(parents=True)
        (path/'child.log').write_bytes(b'original unqualified raw bytes\n')
        records[key] = {'id':key,'status':status,'attempt_id':'attempt-0001','attempt':str(path),
                        'elapsed_ns':SPENT[key],'error':'original Storage' if key == T else 'original stop'}
    cells = {key: row['elapsed_ns'] for key, row in records.items()}
    used = sum(cells.values())+100*10**9
    budget = {'schema':'slice30-gd-budget/v1','used_ns':used,'shared_ns':used+700*10**9,
              'retained_bytes':2*1024**3,'storage_limit':2*1024**3+500*1024**2,
              'cell_used_ns':cells,'common_used_ns':{'G':100*10**9},'inflight':None}
    review = {'verdict':'ACCEPT','reviewer':'literal ordinary replacement review','subject':'exact failed/interrupted pair',
        'n':100000,'mode':'overlap','checkpoint':{'used_ns':used,'retained_bytes':budget['retained_bytes']},
        'prefix_attempts':{cell['id']:records[cell['id']]['attempt_id'] for cell in SPEC['cells'][:12]},
        'qualified_current_attempts':{key:{'attempt_id':records[key]['attempt_id'],'attempt':records[key]['attempt']}
                                     for key in ('d-c-overlap-n100000-r0','d-t-overlap-n100000-r0')},
        'replacements':{key:{'record':copy.deepcopy(records[key]),'charged_ns':SPENT[key],
                             'next_attempt_id':'attempt-0002'} for key in (T,C)},
        'bounds':{layer:{'time_ns':seconds*10**9,'active_bytes':200*1024**2}
                  for layer,seconds in (('T',300),('C',400))}}
    return records,budget,review


class DReplacementTests(unittest.TestCase):
    def test_only_two_pending_members_are_funded_with_exact_cumulative_remainders(self):
        with tempfile.TemporaryDirectory() as directory:
            records,budget,review = fixture(Path(directory))
            decision = reviewed_quartet(SPEC,records,budget,review)
            self.assertEqual(decision['status'],'admit','qualified existing members were forecast as new replacement work')
            self.assertEqual(decision.get('pending_cells'),[T,C])
            self.assertEqual(decision.get('planned_new_time_ns'),700*10**9)
            self.assertEqual(decision.get('planned_new_bytes'),400*1024**2)
            self.assertEqual(decision.get('remaining_ns'),{T:1480608204946,C:1457528171766})

    def test_selected_failed_child_keeps_old_history_stops_second_and_consumes_attempt_once(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)/'attempts'
            records,budget,review = fixture(root)
            old = copy.deepcopy(records)
            budget['shared_ns'] += 2000*10**9
            budget['storage_limit'] = 16*1024**3
            marker = Path(directory)/'dispatches'
            code = '\n'.join([
                'import json,pathlib,sys',
                'cell=json.loads(pathlib.Path(sys.argv[1]).read_text())',
                f'with pathlib.Path({str(marker)!r}).open("a") as stream: stream.write(cell["id"]+"\\n")',
                '(pathlib.Path(sys.argv[2])/"result.json").write_text(json.dumps({"complete":False,"error":"literal replacement Storage"}))',
            ])
            rows = run_supervised_stage(SPEC,records,budget,root,[sys.executable,'-c',code],
                                       initial=False,review=review,scientific=True)
            self.assertTrue(marker.exists(),'explicit replacement never dispatched its selected fresh child')
            self.assertEqual(marker.read_text().splitlines(),[T])
            self.assertEqual(records[T]['attempt_id'],'attempt-0002')
            self.assertEqual(records[T]['status'],'failed')
            self.assertEqual(records[T]['prior_attempts'],[old[T]])
            self.assertEqual(records[C],old[C])
            self.assertGreater(budget['cell_used_ns'][T],SPENT[T])
            self.assertEqual(budget['cell_used_ns'][C],SPENT[C])
            for key,row in old.items():
                if key != T:
                    self.assertEqual(records[key],row)
            self.assertEqual((root/T/'attempt-0001'/'child.log').read_bytes(),b'original unqualified raw bytes\n')
            config=json.loads((root/T/'attempt-0002'/'cell.json').read_text())
            self.assertLessEqual(config['remaining_cell_ms'],1480608)
            self.assertEqual(len(rows),24)
            before=copy.deepcopy(budget)
            run_supervised_stage(SPEC,records,budget,root,[sys.executable,'-c',code],
                                 initial=False,review=review,scientific=True)
            self.assertEqual(marker.read_text().splitlines(),[T])
            self.assertFalse((root/T/'attempt-0003').exists())
            self.assertEqual(budget,before)

    def test_exact_old_current_and_remaining_admission_refuse_before_child_preparation(self):
        for change in ('old-reference','qualified-pin','remaining-time'):
            with self.subTest(change=change),tempfile.TemporaryDirectory() as directory:
                root=Path(directory)/'attempts'
                records,budget,review=fixture(root)
                budget['shared_ns'] += 2000*10**9
                budget['storage_limit']=16*1024**3
                if change == 'old-reference':
                    review['replacements'][T]['record']['error']='different original failure'
                elif change == 'qualified-pin':
                    review['qualified_current_attempts']['d-t-overlap-n100000-r0']['attempt_id']='different'
                else:
                    budget['cell_used_ns'][T]=1200*10**9
                    review['replacements'][T]['charged_ns']=1200*10**9
                decision=reviewed_quartet(SPEC,records,budget,review)
                self.assertNotEqual(decision['status'],'admit','replacement admission ignored original identity or cumulative remainder')
                self.assertFalse((root/T/'attempt-0002').exists())


if __name__ == '__main__':
    unittest.main()
