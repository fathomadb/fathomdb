"""An actual failed D scientific child must prevent later fresh preparation."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from d_driver import run_supervised_stage


class DFailFastTests(unittest.TestCase):
    def test_first_failed_actual_child_prevents_next_member_and_retains_all24(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / 'dispatches'
            spec = json.loads(Path(__file__).with_name('d-cells.json').read_text())
            budget = {'used_ns': 100, 'shared_ns': 14400*10**9,
                      'retained_bytes': 0, 'storage_limit': 16*1024**3,
                      'cell_used_ns': {}, 'inflight': None}
            code = '\n'.join([
                'import json,pathlib,sys',
                'cell=json.loads(pathlib.Path(sys.argv[1]).read_text())',
                f'with pathlib.Path({str(marker)!r}).open("a") as stream: stream.write(cell["id"]+"\\n")',
                '(pathlib.Path(sys.argv[2])/"result.json").write_text(json.dumps({"complete":False,"error":"literal Storage"}))',
            ])
            records = {}
            rows = run_supervised_stage(spec, records, budget, root/'attempts',
                [sys.executable, '-c', code], scientific=True)
            self.assertEqual(marker.read_text().splitlines(), ['d-c-held-n10000-r0'],
                             'failed scientific cell dispatched a later quartet member')
            self.assertEqual(rows[0]['status'], 'failed')
            self.assertEqual([row['id'] for row in rows], [cell['id'] for cell in spec['cells']])
            self.assertTrue(all(row['status'] == 'not_run_failed_prefix' for row in rows[1:4]))
            self.assertEqual(set(records), {'d-c-held-n10000-r0'})
            before = dict(records)
            run_supervised_stage(spec, records, budget, root/'attempts',
                [sys.executable, '-c', code], scientific=True)
            self.assertEqual(marker.read_text().splitlines(), ['d-c-held-n10000-r0'])
            self.assertEqual(records, before)


if __name__ == '__main__':
    unittest.main()
