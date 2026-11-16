"""Explicit delegated public one-case routing control with a harmless executable."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class DDiagnosticEntryTests(unittest.TestCase):
    def test_selected_public_entry_starts_one_owned_child_without_scientific_credit(self):
        outer = Path(os.environ['S30_TASK'])
        with tempfile.TemporaryDirectory(prefix='d-diagnostic-entry-', dir=outer/'tmp') as directory:
            task = Path(directory)
            allocation = task/'allocation'
            allocation.mkdir()
            root = allocation/'diagnostic'
            ledger = allocation/'g-budget.json'
            ledger.write_text(json.dumps({'schema':'slice30-gd-budget/v1','used_ns':123,
                                         'cell_used_ns':{},'common_used_ns':{},'inflight':None}))
            profile = task/'profile.json'
            profile.write_text(json.dumps({'schema':'slice30-d-budget/v1','status':'APPROVED',
                'ledger':str(ledger),'whole_cell_ns':1500*10**9,'total_shared_ns':3600*10**9,
                'total_shared_retained_bytes':1024**3,'reserved_other_work_ns':0,'reserved_other_work_bytes':0}))
            marker = task/'child-observed'
            binary = task/'harmless-diagnostic'
            binary.write_text('\n'.join([
                '#!'+sys.executable,
                'import json,pathlib,sys',
                'cell=json.loads(pathlib.Path(sys.argv[1]).read_text())',
                'assert cell["n"]==100000 and cell["layer"]=="T" and cell["mode"]=="overlap"',
                'assert 0 < cell["remaining_cell_ms"] < 1500000',
                f'pathlib.Path({str(marker)!r}).write_text("started")',
                '(pathlib.Path(sys.argv[2])/"result.json").write_text(json.dumps({"scope":"private D SQLite-log diagnostic","complete":False,"qualifies_natural":False}))',
            ])+'\n')
            binary.chmod(0o700)
            options = task/'options.json'
            options.write_text(json.dumps({'root':str(root),'allocation':str(allocation),
                                           'binary':str(binary),'profile':str(profile)}))
            runner = Path(__file__).with_name('d_runner.py')
            child = subprocess.run([sys.executable,str(runner),'diagnostic','--options',str(options)],
                env=dict(os.environ,S30_TASK=str(task),AGENT_LONG='1'),capture_output=True,text=True,timeout=10)
            self.assertTrue(marker.exists(), 'selected public diagnostic entry never started its owned child')
            self.assertEqual(child.returncode, 0, child.stderr)
            result = json.loads(child.stdout)
            self.assertFalse(result['complete'])
            self.assertFalse(result['qualifies_natural'])
            self.assertTrue(result['guardian']['diagnostic_memory_complete'])
            self.assertFalse((root/'d-state.json').exists())
            final = json.loads(ledger.read_text())
            self.assertEqual(final['used_ns']-123,final['common_used_ns']['D'])
            self.assertEqual(final['cell_used_ns'],{})
            self.assertIsNone(final['inflight'])
            self.assertIsNone(final['d_guardian'])
            self.assertFalse((task/'g-containment-state.json').exists())


if __name__ == '__main__':
    unittest.main()
