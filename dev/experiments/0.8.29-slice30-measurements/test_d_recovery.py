"""Historical D stop conservation and interrupted publication; literal nanoseconds."""

import json
from pathlib import Path
import tempfile
import unittest

from d_recovery import recover


class DRecoveryTests(unittest.TestCase):
    def fixture(self, task):
        allocation = task / 'allocation'
        root = allocation / 'd'
        root.mkdir(parents=True)
        prefix = {'d-c-held-n10000-r0': 'attempt-0001'}
        owner = {'pid': 71001, 'start': 71, 'started_monotonic_ns': 100,
                 'controller_pid': 71002, 'controller_start': 72, 'root': str(root)}
        active = {'cell': 'd-c-overlap-n100000-r1', 'started_monotonic_ns': 180}
        boundary = {'schema': 'slice30-g-containment/v1', 'status': 'active',
                    'guardian_pid': 71001, 'guardian_start': 71, 'name': 'fathomdb-s30-g-original',
                    'path': '/sys/fs/cgroup/original.service/fathomdb-s30-g-original',
                    'relative_path': '/original.service/fathomdb-s30-g-original',
                    'identity': {'device': 31, 'inode': 900}}
        (task / 'g-containment-state.json').write_text(json.dumps(boundary))
        prior = root / 'guardians' / 'prior' / 'guardian.json'
        prior.parent.mkdir(parents=True)
        prior.write_text(json.dumps({'cleanup_uncertain': False, 'exit': 0,
                                     'accounting': {'charged_ns': 40}}))
        attempt = root / 'attempts' / active['cell'] / 'attempt-0001'
        attempt.mkdir(parents=True)
        (attempt / 'child.log').write_text('retained interrupted output')
        records = {'d-c-held-n10000-r0': {'id': 'd-c-held-n10000-r0', 'attempt_id': 'attempt-0001',
                    'status': 'complete', 'elapsed_ns': 30},
                   'd-t-overlap-n100000-r1': {'id': 'd-t-overlap-n100000-r1', 'attempt_id': 'attempt-0001',
                    'status': 'failed', 'error': 'Storage', 'elapsed_ns': 60}}
        (root / 'd-state.json').write_text(json.dumps(records))
        (root / 'd-config.json').write_text(json.dumps({'review': {'prefix_attempts': prefix}}))
        budget = {'schema': 'slice30-gd-budget/v1', 'used_ns': 1070,
                  'cell_used_ns': {'g-prior': 900, 'd-c-held-n10000-r0': 30,
                                   'd-t-overlap-n100000-r1': 60},
                  'common_used_ns': {'M': 70, 'D': 20}, 'inflight': active,
                  'd_guardian': owner, 'm_guardian': None, 's50_guardian': None,
                  'closed_guardian': None}
        (allocation / 'g-budget.json').write_text(json.dumps(budget))
        options = {'allocation': str(allocation), 'root': str(root), 'guardian': owner,
                   'inflight': active, 'boundary': boundary, 'unit': 'original.service',
                   'journal': {'boot_id': 'same-boot', 'invocation': 'original',
                               'start_ns': 99, 'stop_ns': 200},
                   'prefix_attempts': prefix, 'prior_guardians': [str(prior)],
                   'attempt_id': 'attempt-0001'}
        return allocation, root, options, records

    def test_historical_stop_recovers_cell_common_and_preserves_failed_prefix_without_idle(self):
        with tempfile.TemporaryDirectory() as directory:
            task = Path(directory)
            allocation, root, options, before = self.fixture(task)
            result = recover(options, task=task, observe=lambda _: None, clock=lambda: 1000000)
            self.assertTrue(result['complete'], 'terminated original D custody was not recovered')
            budget = json.loads((allocation / 'g-budget.json').read_text())
            self.assertEqual(budget['used_ns'], 1100)
            self.assertEqual(budget['cell_used_ns'], {'g-prior': 900, 'd-c-held-n10000-r0': 30,
                'd-t-overlap-n100000-r1': 60, 'd-c-overlap-n100000-r1': 20})
            self.assertEqual(budget['common_used_ns'], {'M': 70, 'D': 30})
            self.assertIsNone(budget['inflight'])
            self.assertIsNone(budget['d_guardian'])
            records = json.loads((root / 'd-state.json').read_text())
            for key, value in before.items():
                self.assertEqual(records[key], value)
            stopped = records['d-c-overlap-n100000-r1']
            self.assertEqual(stopped['status'], 'interrupted')
            self.assertEqual(stopped['elapsed_ns'], 20)
            self.assertEqual(stopped['attempt_id'], 'attempt-0001')
            self.assertFalse((task / 'g-containment-state.json').exists())
            self.assertEqual((Path(stopped['attempt']) / 'child.log').read_text(), 'retained interrupted output')
            self.assertEqual(len(list((root / 'guardians').glob('*/guardian.json'))), 1)

    def test_interrupted_publications_resume_without_duplicate_historical_charge(self):
        for stage in ('budget', 'state', 'marker', 'complete'):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as directory:
                task = Path(directory)
                allocation, root, options, _ = self.fixture(task)
                def interrupt(current):
                    if current == stage:
                        raise RuntimeError('literal interrupted publication')
                with self.assertRaisesRegex(RuntimeError, 'interrupted publication'):
                    recover(options, task=task, observe=lambda _: None, clock=lambda: 1000000,
                            after_publish=interrupt)
                result = recover(options, task=task, observe=lambda _: None, clock=lambda: 9000000)
                self.assertTrue(result['complete'])
                final = json.loads((allocation / 'g-budget.json').read_text())
                self.assertEqual(final['used_ns'], 1100)
                self.assertEqual(final['cell_used_ns']['d-c-overlap-n100000-r1'], 20)
                self.assertEqual(final['common_used_ns']['D'], 30)
                self.assertEqual(json.loads((root / 'd-state.json').read_text())['d-t-overlap-n100000-r1']['status'], 'failed')


if __name__ == '__main__':
    unittest.main()
