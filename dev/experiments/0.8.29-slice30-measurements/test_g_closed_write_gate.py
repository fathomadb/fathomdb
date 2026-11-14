"""Closed write admission/order protocol with real harmless children and held D/M costs."""

from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

from g_campaign import make_g_plan, run_g_plan
from test_g_closed_modes import harmless, references
from test_g_closed_plan import SPEC

CAL = {"root": "protocol-calibration", "source_sha": "protocol-source", "campaign_uuid": "protocol-cal-uuid"}


def setup():
    spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-write-matrix",
                _calibration_origin=CAL)
    plan = make_g_plan(spec)
    reused = references()
    r1 = next(cell for cell in plan if cell["id"] == "g-r-n100000-r1")
    reused[r1["id"]] = {"id": r1["id"], "cell": r1, "status": "reused_complete",
        "origin": {"root": "protocol-r1", "source_sha": "protocol-r1-source", "campaign_uuid": "protocol-r1-uuid"}}
    review = {"schema": "slice30-closed-write-review/v1", "admission_scope": "G-only",
        "review": {"verdict": "ACCEPT", "reviewer": "protocol-control", "subject": "protocol-frozen-cost-table"},
        "calibration": CAL, "r1": reused[r1["id"]]["origin"],
        "limits": {"campaign_seconds": 14400, "active_bytes": 16*1024**3},
        "observation_wait_ms": 50, "combined_feasibility": "UNKNOWN",
        "other_phases": [{"phase": phase, "status": "HELD_COST_UNAVAILABLE",
                          "forecast_ns": None, "new_bytes": None} for phase in ("D", "M")],
        "g_table": {cell["id"]: {"cell": cell, "prediction": {"allowed": True, "total_ns": 1, "new_bytes": 1}}
                    for cell in plan if cell["stage"] == "G-W"}}
    spec["_reviewed_costs"] = review
    return spec, reused


class GClosedWriteGateTests(unittest.TestCase):
    def test_boolean_review_does_not_approve_actual_child_dispatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = setup()
            spec["_reviewed_costs"] = True
            rows = run_g_plan(spec, root / "results", harmless(root / "forbidden"),
                              allocation_root=root, reused=reused)
            self.assertFalse((root / "forbidden").exists(), "boolean review bypassed actual reviewed-cost bindings")
            self.assertTrue(all(row["status"] == "not_run_prerequisite"
                                for row in rows if row["cell"]["stage"] == "G-W"))

    def test_write_mode_cannot_backfill_unqualified_r1(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = setup()
            reused.pop("g-r-n100000-r1")
            run_g_plan(spec, root / "results", harmless(root / "forbidden"), allocation_root=root, reused=reused)
            self.assertFalse((root / "forbidden").exists(), "write mode silently performed the separate R1 stage")

    def test_explicit_held_other_costs_preserve_required_write_order_and_resume(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = setup()
            original = deepcopy(reused)
            rows = run_g_plan(spec, root / "results", harmless(root / "calls"), allocation_root=root, reused=reused)
            calls = (root / "calls").read_text().splitlines()
            self.assertEqual(len(calls), 72)
            self.assertEqual(calls[:2], ["g-w-b1-s1-engine-text-r0", "g-w-b1-s1-engine-hybrid-r0"])
            self.assertEqual(calls[-2:], ["g-w-b1-s1-engine-hybrid-r1", "g-w-b1-s1-engine-text-r1"])
            self.assertEqual(len(rows), 84)
            self.assertEqual(sum(row["status"] == "reused_complete" for row in rows), 12)
            self.assertEqual(sum(row["status"] == "complete" for row in rows), 72)
            for identifier, row in original.items():
                self.assertEqual(next(value for value in rows if value["id"] == identifier), row)
            run_g_plan(spec, root / "results", harmless(root / "calls"), allocation_root=root, reused=reused)
            self.assertEqual(len((root / "calls").read_text().splitlines()), 72)
            self.assertEqual(spec["_reviewed_costs"]["combined_feasibility"], "UNKNOWN")
            self.assertIsNone(json.loads((root / "g-budget.json").read_text())["inflight"])
