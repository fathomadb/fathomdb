"""Write-matrix entry requires actual reviewed costs, not just a completed R1."""

from pathlib import Path
import tempfile
import unittest

from g_campaign import make_g_plan, run_g_plan
from test_g_closed_modes import harmless, references
from test_g_closed_plan import SPEC


class GClosedReviewTests(unittest.TestCase):
    def test_completed_r1_without_cost_review_cannot_dispatch_any_write(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "forbidden-calls"
            spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-write-matrix")
            reused = references()
            r1 = next(cell for cell in make_g_plan(spec) if cell["id"] == "g-r-n100000-r1")
            reused[r1["id"]] = {"id": r1["id"], "cell": r1, "status": "reused_complete",
                                "origin": {"root": "protocol-qualified-r1"}}
            rows = run_g_plan(spec, root / "results", harmless(calls), allocation_root=root,
                              reused=reused)
            self.assertFalse(calls.exists(), "completed R1 auto-approved write costs and dispatched children")
            self.assertEqual(len(rows), 84)
            writes = [row for row in rows if row["cell"]["stage"] == "G-W"]
            self.assertEqual(len(writes), 72)
            self.assertTrue(all(row["status"] == "not_run_prerequisite"
                                and row["reason"] == "reviewed_write_costs_required" for row in writes))
