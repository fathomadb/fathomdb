"""Explicit delegated raw memory evidence before/after a harmless diagnostic child."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

from d_runner import run_guarded


class DDiagnosticMemoryTests(unittest.TestCase):
    def test_outer_memory_counters_survive_child_exit_and_stay_inside_charge(self):
        task = Path(os.environ['S30_TASK'])
        with tempfile.TemporaryDirectory(prefix='d-memory-', dir=task/'tmp') as directory:
            allocation = Path(directory)
            ledger = allocation/'g-budget.json'
            ledger.write_text(json.dumps({'schema':'slice30-gd-budget/v1', 'used_ns':123,
                                         'cell_used_ns':{}, 'common_used_ns':{}, 'inflight':None}))
            profile = {'schema':'slice30-d-budget/v1', 'status':'APPROVED', 'ledger':str(ledger),
                'whole_cell_ns':1500*10**9, 'total_shared_ns':60*10**9,
                'total_shared_retained_bytes':1024**3,
                'reserved_other_work_ns':0, 'reserved_other_work_bytes':0}
            result = run_guarded([sys.executable, '-c', 'import time;time.sleep(.02)'],
                allocation=allocation, root=allocation/'result', task=task, profile=profile,
                diagnostic_memory=True)
            self.assertEqual(result['exit'], 0)
            self.assertIn('diagnostic_memory_before', result,
                          'owning guardian omitted raw memory counters before diagnostic work')
            self.assertIn('diagnostic_memory_after', result)
            self.assertTrue(result['diagnostic_memory_complete'])
            for key in ('diagnostic_memory_before', 'diagnostic_memory_after'):
                sample = result[key]
                self.assertEqual(sample['raw']['memory.max'], '4294967296')
                self.assertEqual(sample['raw']['memory.swap.max'], '0')
                self.assertIn('oom_kill ', sample['raw']['memory.events'])
                self.assertGreaterEqual(int(sample['raw']['memory.peak']), int(sample['raw']['memory.current']))
            self.assertFalse(result['cleanup_uncertain'])
            self.assertTrue(result['boundary']['removed_exact_empty_boundary'])
            budget = json.loads(ledger.read_text())
            self.assertEqual(budget['used_ns']-123, result['accounting']['charged_ns'])
            self.assertEqual(budget['used_ns']-123, budget['common_used_ns']['D'])
            self.assertIsNone(budget['d_guardian'])


if __name__ == '__main__':
    unittest.main()
