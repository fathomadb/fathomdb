"""Hand four-cell retained-storage math; all bytes are synthetic unit inputs."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from d_controller import run_required
from test_d_controller import GIB, SPEC, budget, literal_records


class DQuartetStorageTests(unittest.TestCase):
    def test_four_individually_fitting_12gib_predictions_refuse_before_preparation(self):
        records = literal_records(100000, seconds=100, peak_bytes=2 * GIB)
        ledger = budget()
        before = dict(ledger)
        with tempfile.TemporaryDirectory() as temporary:
            marker = Path(temporary) / "forbidden-large-preparation"
            (Path(temporary) / "unit-ledger.json").write_text(__import__("json").dumps(ledger))

            def prepare(cell):
                if cell["n"] == 300000 and cell["mode"] == "held":
                    subprocess.run([sys.executable, "-c",
                        "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('dispatched')",
                        str(marker)], check=True, timeout=2)
                return {"status": "incomplete", "unit_diagnostic": True}

            rows = run_required(SPEC, records, ledger, prepare)
            self.assertFalse(marker.exists(), "48GiB quartet dispatched under the shared16GiB bound")
            held = [row for row in rows if row["id"].startswith(("d-c-held-n300000", "d-t-held-n300000"))]
            self.assertEqual(len(held), 4)
            self.assertTrue(all(row["status"] == "not_run_cap_feasibility" for row in held))
            for layer in ("C", "T"):
                self.assertEqual(held[0]["predictions"][layer], {"time_ns": 600 * 10**9, "active_bytes": 12 * GIB})
            self.assertEqual(held[0]["quartet_bytes"], 48 * GIB)
            self.assertEqual(ledger, before, "prospective refusal reset or changed prior charge/retention")


if __name__ == "__main__":
    unittest.main()
