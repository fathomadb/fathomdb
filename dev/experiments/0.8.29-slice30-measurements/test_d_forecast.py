"""Literal conservative D time fixtures; no observed workload or feasibility claim."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from d_controller import run_required
from test_d_controller import SPEC, budget, literal_records


class DCellTimeTests(unittest.TestCase):
    def test_1200_seconds_and_exact_900_seconds_refuse_before_real_larger_child(self):
        for seconds in (200, 150):
            with self.subTest(lower_seconds=seconds), tempfile.TemporaryDirectory() as temporary:
                marker = Path(temporary) / "forbidden-large-preparation"
                records = literal_records(100000, seconds=seconds)

                def prepare(cell):
                    if cell["n"] == 300000 and cell["mode"] == "held":
                        subprocess.run([sys.executable, "-c",
                            "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('dispatched')",
                            str(marker)], check=True, timeout=2)
                    return {"status": "incomplete", "unit_diagnostic": True}

                rows = run_required(SPEC, records, budget(), prepare)
                self.assertFalse(marker.exists(), "at-or-over-900s prediction dispatched larger work")
                held = [row for row in rows if row["id"].startswith(("d-c-held-n300000", "d-t-held-n300000"))]
                self.assertEqual(len(held), 4)
                self.assertTrue(all(row["status"] == "not_run_cap_feasibility" for row in held))
                self.assertEqual(held[0]["predictions"]["C"]["time_ns"], seconds * 6 * 10**9)
                self.assertEqual(held[0]["predictions"]["T"]["time_ns"], seconds * 6 * 10**9)


if __name__ == "__main__":
    unittest.main()
