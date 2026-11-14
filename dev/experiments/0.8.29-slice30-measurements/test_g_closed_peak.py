"""Real SQLite WAL peak observation is retained after normal database close."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from g_campaign import run_g_plan
from test_g_closed_modes import references
from test_g_closed_plan import SPEC


class GClosedPeakTests(unittest.TestCase):
    def test_actual_live_wal_peak_survives_normal_sqlite_close(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            command = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sqlite3,sys,time",
                "out=pathlib.Path(sys.argv[2]);store=out/'store.sqlite';db=sqlite3.connect(store)",
                "db.execute('PRAGMA journal_mode=WAL');db.execute('CREATE TABLE data(body BLOB)')",
                "db.execute('INSERT INTO data VALUES(?)',(b'x'*2*1024**2,));db.commit()",
                "live=sum(p.stat().st_size for p in out.glob('store.sqlite*') if p.is_file())",
                "time.sleep(1.3);db.close()",
                "(out/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True,live_store_bytes=live)))",
            ])]
            spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-read-r1")
            rows = run_g_plan(spec, root / "results", command, allocation_root=root, reused=references())
            row = next(row for row in rows if row["id"] == "g-r-n100000-r1")
            self.assertEqual(row["status"], "complete")
            facts = json.loads(Path(row["attempt"], "result.json").read_text())
            self.assertGreaterEqual(row.get("peak_store_bytes", 0), facts["live_store_bytes"],
                                    "live WAL/store peak vanished from final cost evidence")
            final_store = sum(p.stat().st_size for p in Path(row["attempt"]).glob("store.sqlite*") if p.is_file())
            self.assertGreater(row["peak_store_bytes"], final_store)
            self.assertGreaterEqual(row["peak_retained_bytes"], row["peak_store_bytes"])
