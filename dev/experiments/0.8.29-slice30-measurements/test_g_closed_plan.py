"""Prospective closed-copy plans; no store, native or GPU execution."""

import json
from pathlib import Path
import unittest

from g_campaign import make_g_plan

SPEC = json.loads(Path(__file__).with_name("g-cells.json").read_text())
ORIGINAL = {"g0-smoke", "g1-n10000", "g1-n100000", "g-r-n10000-r0",
            "g-r-n10000-r1", "g-r-n100000-r0", "g-o-branches",
            "g-o-natural-r0-off", "g-o-natural-r0-on", "g-o-natural-r1-on",
            "g-o-natural-r1-off"}


class GClosedPlanTests(unittest.TestCase):
    def test_only_unmeasured_cells_use_copy_and_prospective_1500_seconds(self):
        ordinary = make_g_plan(SPEC)
        successor = make_g_plan(dict(SPEC, preparation="closed-copy-v1"))
        self.assertEqual(len(successor), 84)
        self.assertEqual([row["id"] for row in successor], [row["id"] for row in ordinary])
        for original, revised in zip(ordinary, successor, strict=True):
            if original["id"] in ORIGINAL:
                self.assertEqual(revised, original)
            else:
                self.assertEqual(revised["cell_seconds"], 1500,
                                 "unmeasured read/write still has the historical900s policy")
                self.assertEqual(revised["preparation"], "closed-copy-v1")
                self.assertTrue(revised["fresh_copy"])
                self.assertFalse(revised["fresh_seed"])
                for key in ("adapters", "routes", "samples", "warmup", "repeat"):
                    self.assertEqual(revised[key], original[key])
        self.assertEqual(sum(row.get("fresh_copy", False) for row in successor), 73)
        self.assertEqual(successor[5]["id"], "g-r-n100000-r1")
        self.assertEqual(successor[5]["adapters"], ["sdk", "engine"])
        self.assertEqual(successor[5]["routes"], ["embed", "hybrid", "text"])
