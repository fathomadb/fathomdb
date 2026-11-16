"""Explicit fresh-process SQLite-log controls; no D100k execution or ordinary credit."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class DStorageDiagnosticTests(unittest.TestCase):
    def invoke(self, *arguments):
        binary = Path(os.environ['S30_D_DIAGNOSTIC_BINARY']).resolve(strict=True)
        child = subprocess.run([str(binary), *map(str, arguments)], check=True,
                               capture_output=True, text=True, timeout=10)
        return json.loads(child.stdout)

    def test_actual_sqlite_error_reaches_integer_code_callback_in_fresh_process(self):
        with tempfile.TemporaryDirectory(dir=Path(os.environ['S30_TASK'])/'tmp') as directory:
            database = Path(directory)/'real-control.sqlite'
            report = self.invoke('--sqlite-error-control', database)
            self.assertTrue(database.exists())
            self.assertEqual(report['control_error_code'], 1)
            self.assertTrue(any(row['code'] == 1 and 'slice30_d_diagnostic_missing' in row['message']
                                for row in report['sqlite_log']['records']),
                            'actual SQLite statement failure was lost before retained callback evidence')
            self.assertTrue(report['sqlite_log']['capture_complete'])
            self.assertFalse(report['complete'])
            self.assertFalse(report['qualifies_natural'])

    def test_literal_capture_bound_drop_and_utf8_truncation_fail_closed(self):
        report = self.invoke('--bounds-control')
        capture = report['sqlite_log']
        self.assertEqual(len(capture['records']), 64, 'bounded SQLite diagnostic capture is unavailable')
        self.assertEqual(capture['dropped'], 1)
        self.assertEqual(capture['truncated'], 1)
        self.assertEqual(capture['records'][0]['message'], '雪'*341)
        self.assertEqual(capture['records'][0]['code'], 1)
        self.assertFalse(capture['capture_complete'])
        self.assertFalse(report['complete'])
        self.assertFalse(report['qualifies_natural'])


    def native_case(self, config):
        with tempfile.TemporaryDirectory(dir=Path(os.environ['S30_TASK'])/'tmp') as directory:
            root = Path(directory)
            selected, output = root/'cell.json', root/'output'
            output.mkdir()
            selected.write_text(json.dumps(config))
            child = subprocess.run([os.environ['S30_D_DIAGNOSTIC_BINARY'], str(selected), str(output)],
                env=dict(os.environ, AGENT_LONG='1'), capture_output=True, text=True, timeout=10)
            self.assertEqual(child.returncode, 1)
            self.assertTrue((output/'result.json').exists(), 'private fixed-case entry did not retain diagnostic result')
            self.assertFalse((output/'store.sqlite').exists(), 'source admission control created a measurement store')
            return json.loads((output/'result.json').read_text())

    def test_fixed100k_entry_retains_close_reserve_without_store_preparation(self):
        report = self.native_case({'id':'d-t-overlap-n100000-r1','n':100000,'mode':'overlap',
            'repeat':1,'layer':'T','cell_ns':1500*10**9,'remaining_cell_ms':1,'attempt_id':'source-control-only'})
        self.assertIn('close reserve', report['workload_error'])
        self.assertFalse(report['complete'])
        self.assertFalse(report['qualifies_natural'])
        self.assertTrue(report['sqlite_log']['capture_complete'])

    def test_other_fixed_cell_size_or_mode_refuses_before_sqlite_open(self):
        for n, mode, cell_ns in ((10000,'held',900*10**9),(100000,'held',1500*10**9)):
            with self.subTest(n=n,mode=mode):
                report = self.native_case({'id':f'd-t-{mode}-n{n}-r0','n':n,'mode':mode,
                    'repeat':0,'layer':'T','cell_ns':cell_ns,'remaining_cell_ms':1,'attempt_id':'source-control-only'})
                self.assertIn('only fixed100k T overlap', report['workload_error'])
                self.assertFalse(report['complete'])
                self.assertFalse(report['qualifies_natural'])


if __name__ == '__main__':
    unittest.main()
