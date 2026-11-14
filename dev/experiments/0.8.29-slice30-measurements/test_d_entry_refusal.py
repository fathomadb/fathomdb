"""Refuse exhausted temporary D budget before even the initial harmless child."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from d_controller import run_initial_quartet
from test_d_controller import SPEC, budget


class DEntryRefusalTests(unittest.TestCase):
    def test_exhausted_shared_time_or_retention_never_dispatches_initial_child(self):
        for exhausted in ("time", "storage"):
            with self.subTest(exhausted=exhausted), tempfile.TemporaryDirectory() as temporary:
                ledger = budget()
                if exhausted == "time":
                    ledger["used_ns"] = ledger["shared_ns"]
                else:
                    ledger["retained_bytes"] = ledger["storage_limit"]
                before = dict(ledger)
                marker = Path(temporary) / "forbidden-initial-preparation"

                def prepare(cell):
                    subprocess.run([sys.executable, "-c",
                        "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('dispatched')",
                        str(marker)], check=True, timeout=2)
                    return {"status": "incomplete", "unit_diagnostic": True}

                rows = run_initial_quartet(SPEC, {}, ledger, prepare)
                self.assertFalse(marker.exists(), "exhausted shared allocation dispatched initial preparation")
                self.assertEqual(len(rows), 24)
                self.assertTrue(all(row["status"] == "not_run_cap_feasibility" for row in rows[:4]))
                self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))
                self.assertEqual(ledger, before)


if __name__ == "__main__":
    unittest.main()
