"""Explicit delegated real guardian TERM control with one harmless nested child."""

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest


class DCancelTests(unittest.TestCase):
    def test_term_finalizes_owned_boundary_accounting_and_interrupted_state(self):
        outer = Path(os.environ['S30_TASK'])
        with tempfile.TemporaryDirectory(prefix='d-cancel-', dir=outer/'tmp') as directory:
            task = Path(directory)
            root = task/'allocation'/'d'
            root.mkdir(parents=True)
            ledger = root.parent/'g-budget.json'
            ledger.write_text(json.dumps({'schema': 'slice30-gd-budget/v1', 'used_ns': 123,
                'cell_used_ns': {'g-prior': 101}, 'common_used_ns': {'M': 17}, 'inflight': None}))
            child_ready, output = task/'nested-ready', task/'outcome.json'
            profile = {'schema': 'slice30-d-budget/v1', 'status': 'APPROVED',
                'ledger': str(ledger), 'whole_cell_ns': 900*10**9, 'total_shared_ns': 60*10**9,
                'total_shared_retained_bytes': 1024**3,
                'reserved_other_work_ns': 0, 'reserved_other_work_bytes': 0}
            nested = f'import os,pathlib,time;pathlib.Path({str(child_ready)!r}).write_text(str(os.getpid()));time.sleep(60)'
            controller = '\n'.join([
                'import json,pathlib,subprocess,sys,time',
                f'p=pathlib.Path({str(ledger)!r}); b=json.loads(p.read_text())',
                "b['inflight']={'cell':'d-c-held-n10000-r0','started_monotonic_ns':time.monotonic_ns()}",
                'p.write_text(json.dumps(b))',
                f'subprocess.Popen([sys.executable,"-c",{nested!r}],start_new_session=True)',
                'time.sleep(60)',
            ])
            source = str(Path(__file__).parent.resolve())
            guardian_code = '\n'.join([
                'import json,pathlib,sys', f'sys.path.insert(0,{source!r})',
                'from d_runner import run_guarded',
                f'result=run_guarded([sys.executable,"-c",{controller!r}],allocation={str(root.parent)!r},profile={profile!r},root={str(root)!r},task={str(task)!r})',
                f'pathlib.Path({str(output)!r}).write_text(json.dumps(result))',
            ])
            guardian = subprocess.Popen([sys.executable, '-c', guardian_code])
            try:
                until = time.monotonic()+5
                while not child_ready.exists() and guardian.poll() is None and time.monotonic()<until:
                    time.sleep(.01)
                self.assertTrue(child_ready.exists(), 'TERM control never entered actual owned work')
                guardian.send_signal(signal.SIGTERM)
                guardian.wait(timeout=10)
                self.assertTrue(output.exists(), 'SIGTERM killed guardian before final cleanup/accounting publication')
                outcome = json.loads(output.read_text())
                self.assertTrue(outcome['cancelled'])
                self.assertEqual(outcome['exit'], 143)
                self.assertFalse(outcome['cleanup_uncertain'])
                self.assertTrue(outcome['boundary']['removed_exact_empty_boundary'])
                self.assertFalse((task/'g-containment-state.json').exists())
                budget = json.loads(ledger.read_text())
                self.assertIsNone(budget['d_guardian'])
                self.assertIsNone(budget['inflight'])
                self.assertEqual(budget['used_ns']-123,
                    budget['cell_used_ns']['d-c-held-n10000-r0']+budget['common_used_ns']['D'])
                self.assertEqual(budget['cell_used_ns']['g-prior'], 101)
                self.assertEqual(budget['common_used_ns']['M'], 17)
                state = json.loads((root/'d-state.json').read_text())
                self.assertEqual(state['d-c-held-n10000-r0']['status'], 'interrupted')
                fields = Path(f'/proc/{int(child_ready.read_text())}/stat')
                if fields.exists():
                    self.assertIn(fields.read_text().rsplit(') ', 1)[1].split()[0], ('Z', 'X'))
            finally:
                if guardian.poll() is None:
                    guardian.kill()
                    guardian.wait(timeout=3)
                budget = json.loads(ledger.read_text())
                owner = budget.get('d_guardian') or {}
                for pid in (owner.get('controller_pid'), int(child_ready.read_text()) if child_ready.exists() else None):
                    if pid:
                        try:
                            os.kill(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass


if __name__ == '__main__':
    unittest.main()
