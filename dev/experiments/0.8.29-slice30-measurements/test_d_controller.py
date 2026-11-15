"""Hand D feasibility fixtures and harmless-child refusal; no measured evidence."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from d_controller import run_required

GIB = 1024**3
SPEC = json.loads(Path(__file__).with_name("d-cells.json").read_text())


def literal_records(*sizes, seconds=10, peak_bytes=256 * 1024**2):
    return {cell["id"]: {
        **cell, "status": "complete", "error_count": 0,
        "physical_count": cell["n"], "closed": True,
        "observations_complete": True, "dropped_records": 0,
        "actual_overlap": True, "elapsed_ns": seconds * 10**9,
        "peak_bytes": peak_bytes, "unit_diagnostic": True,
    } for cell in SPEC["cells"] if cell["n"] in sizes}


def budget():
    return {"used_ns": 1845130877940, "shared_ns": 14400000000000,
            "retained_bytes": 1903142018, "storage_limit": 16 * GIB}


class DLowerEvidenceTests(unittest.TestCase):
    def test_missing_required_lower_characterization_repeat_refuses_real_preparation_child(self):
        records = literal_records(10000, 100000)
        del records["d-c-held-n100000-r1"]
        with tempfile.TemporaryDirectory() as temporary:
            marker = Path(temporary) / "forbidden-large-preparation"

            def prepare(cell):
                if cell["n"] == 300000 and cell["mode"] == "held":
                    subprocess.run([sys.executable, "-c",
                        "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('dispatched')",
                        str(marker)], check=True, timeout=2)
                return {"status": "incomplete", "unit_diagnostic": True}

            outcomes = run_required(SPEC, records, budget(), prepare)
            self.assertFalse(marker.exists(), "larger preparation ran without both lower C/T repeats")
            self.assertEqual([row["id"] for row in outcomes], [cell["id"] for cell in SPEC["cells"]])
            held = [row for row in outcomes if row["id"].startswith(("d-c-held-n300000", "d-t-held-n300000"))]
            self.assertEqual(len(held), 4)
            self.assertTrue(all(row["status"] == "not_run_lower_evidence" for row in held))


if __name__ == "__main__":
    unittest.main()
