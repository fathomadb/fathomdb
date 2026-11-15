"""Literal2400s/6GiB quartet against consumed shared allowance; unit inputs only."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from d_controller import run_required
from test_d_controller import GIB, SPEC, budget, literal_records


class DSharedTimeTests(unittest.TestCase):
    def test_remaining_2399_seconds_refuses_2400_second_quartet_before_preparation(self):
        records = literal_records(100000, seconds=100, peak_bytes=256 * 1024**2)
        ledger = budget()
        ledger["used_ns"] = 12001 * 10**9
        before = dict(ledger)
        with tempfile.TemporaryDirectory() as temporary:
            marker = Path(temporary) / "forbidden-large-preparation"

            def prepare(cell):
                if cell["n"] == 300000 and cell["mode"] == "held":
                    subprocess.run([sys.executable, "-c",
                        "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('dispatched')",
                        str(marker)], check=True, timeout=2)
                return {"status": "incomplete", "unit_diagnostic": True}

            rows = run_required(SPEC, records, ledger, prepare)
            self.assertFalse(marker.exists(), "2400s quartet dispatched with2399s shared allowance")
            held = [row for row in rows if row["id"].startswith(("d-c-held-n300000", "d-t-held-n300000"))]
            self.assertEqual(len(held), 4)
            self.assertTrue(all(row["status"] == "not_run_cap_feasibility" for row in held))
            self.assertEqual(held[0]["quartet_time_ns"], 2400 * 10**9)
            self.assertEqual(held[0]["quartet_bytes"], 6 * GIB)
            self.assertEqual(held[0]["predictions"]["C"]["active_bytes"], 1536 * 1024**2)
            self.assertEqual(ledger, before, "admission must not reset retained prior charges")


if __name__ == "__main__":
    unittest.main()
