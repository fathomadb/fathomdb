"""Harmless processes exercise C300k guardian limits with literal clock advances."""

from contextlib import ExitStack
import json
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from m_guard import guard
from test_d_held_replacement import CELL, SPENT, fixture
from test_m_calibration_guard import FakeBoundary


class DHeldGuardTests(unittest.TestCase):
    def run_child(self, seconds, *, cumulative_ns=7500000000000):
        with ExitStack() as stack:
            root=Path(stack.enter_context(tempfile.TemporaryDirectory()))
            records,budget,review=fixture(root/'d/attempts')
            selected=review['replacements'][CELL]
            selected['cumulative_limit_ns']=cumulative_ns
            ledger=root/'g-budget.json';ledger.write_text(json.dumps(budget))
            marker=root/'started';origin=time.monotonic_ns()
            code='\n'.join(['import json,os,pathlib,time',f'p=pathlib.Path({str(ledger)!r})',
                'b=json.loads(p.read_text());started=time.monotonic_ns()',
                f'b["d_replacements"][{CELL!r}]={{"selection":{selected!r},"attempt_id":"attempt-0002","attempt":{str(root/"d/attempts"/CELL/"attempt-0002")!r},"started_monotonic_ns":started}}',
                f'b["inflight"]={{"cell":{CELL!r},"started_monotonic_ns":started,"replacement":True,"attempt_id":"attempt-0002","attempt":{str(root/"d/attempts"/CELL/"attempt-0002")!r}}}',
                'q=p.with_suffix(".child-tmp");q.write_text(json.dumps(b));os.replace(q,p)',
                f'pathlib.Path({str(marker)!r}).write_text(str(started))','time.sleep(.25)'])
            real_clock=time.monotonic_ns
            def clock():
                return int(marker.read_text())+seconds*10**9 if marker.exists() else real_clock()
            stack.enter_context(patch('m_guard.TaskBoundary',FakeBoundary))
            stack.enter_context(patch('m_guard.resource_sample',return_value={
                'root_free':1000*1024**3,'task_free':1000*1024**3,'memory_available':100*1024**3,'owned':0}))
            stack.enter_context(patch('m_guard.time.monotonic_ns',side_effect=clock))
            profile={'schema':'slice30-d-budget/v1','status':'APPROVED','ledger':str(ledger),
                'whole_cell_ns':4500000000000,'total_shared_ns':172800000000000,
                'total_shared_retained_bytes':192*1024**3,'reserved_other_work_ns':0,'reserved_other_work_bytes':0}
            result=guard([sys.executable,'-c',code],allocation=root,root=root/'d',task=root,
                         phase='D',profile=profile,started_ns=origin,common_limit_ns=60*10**9)
            self.assertTrue(marker.exists(),'actual harmless C300k controller did not start')
            self.assertFalse(result['cleanup_uncertain'])
            after=json.loads(ledger.read_text())
            self.assertEqual(after['cell_used_ns'][CELL],SPENT+seconds*10**9)
            self.assertIsNone(after.get('d_guardian'))
            return result

    def test_valid_consumed_c300k_gets_fresh4000s_beyond_old_cumulative4500s(self):
        result=self.run_child(4000)
        self.assertEqual(result['exit'],0,'guardian still stops at old1947s cumulative remainder')

    def test_fresh4500s_reserve_still_stops_owned_controller(self):
        result=self.run_child(4490)
        self.assertEqual(result['exit'],124)
        self.assertEqual(result['stop'],'shared cleanup reserve')

    def test_unreviewed_cumulative_limit_does_not_expand_guard(self):
        result=self.run_child(4000,cumulative_ns=7500000000001)
        self.assertEqual(result['exit'],124)


if __name__=='__main__':
    unittest.main()
