"""Real harmless-child primary stop/retry controls, never scientific qualification."""

import copy
import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_pair import drive_members, plan
from m_attempt import run_attempt


COMMAND = [sys.executable, '-c', '''
import json,pathlib,sys
cell=json.loads(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2])
(root/'result.json').write_text(json.dumps({'complete':cell.get('control_complete',True),'runner_only':True}))
''']


def child(cell, root):
    return run_attempt(cell, root / 'results', COMMAND, deadline_ns=time.monotonic_ns()+10*10**9)


class PairRetryTests(unittest.TestCase):
    def fixture(self, root):
        cells = plan()['primary']
        previous = []
        for cell in cells[:32]:
            attempt = root / 'results' / cell['id'] / 'attempt-0001'
            attempt.mkdir(parents=True)
            (attempt / 'result.json').write_text('retained prior native result')
            previous.append({'id': cell['id'], 'cell': cell, 'attempt': str(attempt),
                             'status': 'complete', 'complete': True, 'selection': 'prior-P-selection'})
        failures = []
        for cell in cells[32:34]:
            outcome = child(dict(cell, control_complete=False), root)
            outcome.update(cell=cell, charged_ns=17, selection='old-adoption',
                           guardian_owner={'controller_pid': 123})
            failures.append(outcome)
        previous.extend(failures)
        state = root / 'results/campaign-state.json'
        state.write_text(json.dumps(previous, indent=2)+'\n')
        manifest = {'scope': 'primary', 'plan': plan(), '_active_members': cells[:34]}
        return manifest, previous, failures, state

    def test_primary_stops_before_second_member_after_first_actual_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cells = plan()['primary'][:2]
            manifest = {'scope': 'primary', 'plan': plan(), '_active_members': cells}
            calls = []
            def execute(cell, index):
                calls.append(cell['id'])
                return child(dict(cell, control_complete=False), root)
            rows = drive_members(manifest, root, execute)
            self.assertEqual(calls, [cells[0]['id']], 'primary dispatched another member after failure')
            self.assertEqual(len(rows), 1)
            self.assertFalse((root / 'results' / cells[1]['id']).exists())
            self.assertFalse(rows[0]['complete'])

    def test_failed_member_requires_a_new_explicit_adoption_before_another_child(self):
        for selection in (None, 'old-adoption'):
            with self.subTest(selection=selection), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                manifest, previous, failures, state = self.fixture(root)
                if selection:
                    manifest.update(_selection_path=selection, _selection_record={'guardian_owner': {'pid': 7}})
                before = state.read_bytes()
                calls = []
                def execute(cell, index):
                    calls.append(cell['id'])
                    return child(cell, root)
                with self.assertRaisesRegex(ValueError, 'fresh.*selection|new.*selection'):
                    drive_members(manifest, root, execute)
                self.assertEqual(calls, [])
                self.assertEqual(state.read_bytes(), before)
                self.assertFalse(list((root / 'results').glob('*/attempt-0002')))

    def test_new_selection_retries_fresh_attempts_and_retains_P_rows_failures_and_charges(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, previous, failures, state = self.fixture(root)
            originals = copy.deepcopy(previous)
            old_files = {str(p): p.read_bytes() for row in previous for p in Path(row['attempt']).iterdir()}
            manifest.update(_selection_path='new-adoption', _selection_record={'guardian_owner': {'pid': 7}})
            budget = {'used_ns': 1000, 'cell_used_ns': {row['id']: row['charged_ns'] for row in failures}}
            before = copy.deepcopy(budget)
            calls = []
            def execute(cell, index):
                calls.append(cell['id'])
                result = child(cell, root)
                charge = result['elapsed_ns']
                budget['used_ns'] += charge
                budget['cell_used_ns'][cell['id']] += charge
                result.update(charged_ns=charge, selection='new-adoption')
                return result
            rows = drive_members(manifest, root, execute)
            self.assertEqual(calls, [row['id'] for row in failures])
            self.assertEqual(rows[:32], originals[:32])
            self.assertEqual(json.loads(state.read_text())[:32], originals[:32])
            for old, new in zip(failures, rows[32:], strict=True):
                self.assertEqual(Path(new['attempt']).name, 'attempt-0002')
                self.assertEqual(new['prior_attempts'], [old])
                self.assertEqual(budget['cell_used_ns'][new['id']], before['cell_used_ns'][new['id']]+new['charged_ns'])
            self.assertEqual(budget['used_ns'], before['used_ns']+sum(row['charged_ns'] for row in rows[32:]))
            self.assertEqual({path: Path(path).read_bytes() for path in old_files}, old_files)
