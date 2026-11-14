"""Closed initial D stage stop; real harmless children, no measured execution."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from d_controller import run_initial_quartet
from test_d_controller import SPEC, budget


class DInitialStopTests(unittest.TestCase):
    def test_initial_held_quartet_stops_before_overlap_preparation_with_all24_visible(self):
        with tempfile.TemporaryDirectory() as temporary:
            marker = Path(temporary) / "forbidden-overlap-preparation"
            called = []

            def prepare(cell):
                called.append(cell["id"])
                if cell["mode"] == "overlap":
                    subprocess.run([sys.executable, "-c",
                        "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('dispatched')",
                        str(marker)], check=True, timeout=2)
                return {"status": "incomplete", "unit_diagnostic": True}

            rows = run_initial_quartet(SPEC, {}, budget(), prepare)
            self.assertFalse(marker.exists(), "initial quartet auto-advanced into overlap preparation")
            self.assertEqual(called, ["d-c-held-n10000-r0", "d-t-held-n10000-r0",
                                      "d-t-held-n10000-r1", "d-c-held-n10000-r1"])
            self.assertEqual([row["id"] for row in rows], [cell["id"] for cell in SPEC["cells"]])
            self.assertEqual(len(rows), 24)
            self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[4:]))


if __name__ == "__main__":
    unittest.main()
