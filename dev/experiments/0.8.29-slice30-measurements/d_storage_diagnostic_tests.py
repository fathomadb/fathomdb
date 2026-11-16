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


if __name__ == '__main__':
    unittest.main()
