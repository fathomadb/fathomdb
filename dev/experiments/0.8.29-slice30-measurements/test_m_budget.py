"""Harmless actual children must never start under an absent or infeasible profile."""

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

from m_controller import load_plan, run_budgeted_table


def bounds():
    """Literal source-control costs; these are not estimates for natural MCP work."""
    return {"components_ns": {"copy": 0, "packaging": 0, "open": 0, "first": 0,
                             "warmups": 0, "timed": 10000000, "logging": 0,
                             "persistence": 0, "polling": 0, "idle": 0,
                             "close": 0, "diagnostics": 0, "failed_attempts": 0},
            "new_retained_bytes": 100}


class BudgetRefusalTests(unittest.TestCase):
    """The complete remaining table must fit before preparation or child dispatch."""

    def test_refuse_all_required_rows_before_copy_or_start(self):
        cases = ("missing_profile", "inactive_profile", "missing_component", "shared_time",
                 "complete_table_time", "retained_storage", "cell_time", "unknown_inflight")
        with tempfile.TemporaryDirectory() as temporary:
            for name in cases:
                with self.subTest(name=name):
                    root = Path(temporary) / name
                    allocation = root / "allocation"
                    allocation.mkdir(parents=True)
                    ledger_path = allocation / "g-budget.json"
                    ledger = {"schema": "slice30-gd-budget/v1", "used_ns": 0,
                              "inflight": None, "cell_used_ns": {}}
                    profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
                               "ledger": str(ledger_path), "total_shared_ns": 100000000,
                               "total_shared_retained_bytes": 1048576,
                               "whole_cell_ns": 900000000000}
                    plan = load_plan()[:2]
                    forecasts = {cell["id"]: bounds() for cell in plan}
                    if name == "missing_profile":
                        profile = None
                    elif name == "inactive_profile":
                        profile["status"] = "PROPOSED_NOT_ACTIVE"
                    elif name == "missing_component":
                        del forecasts[plan[1]["id"]]["components_ns"]["close"]
                    elif name == "shared_time":
                        ledger["used_ns"] = 100000000
                    elif name == "complete_table_time":
                        ledger["used_ns"] = 65000000
                    elif name == "retained_storage":
                        (allocation / "retained.raw").write_bytes(b"x" * 1048576)
                    elif name == "cell_time":
                        forecasts[plan[0]["id"]]["components_ns"]["open"] = 451000000000
                        profile["total_shared_ns"] = 10000000000000
                    elif name == "unknown_inflight":
                        ledger["inflight"] = {"cell": "prior-unknown"}
                    ledger_path.write_text(json.dumps(ledger))
                    original = copy.deepcopy(ledger)
                    started = root / "started"
                    prepared = root / "prepared"
                    child = ("import pathlib,sys; pathlib.Path(" + repr(str(started))
                             + ").write_text('started')")
                    result = run_budgeted_table(
                        allocation / "m-results", [sys.executable, "-c", child],
                        profile=profile, allocation=allocation, forecasts=forecasts, plan=plan,
                        prepare=lambda *_: prepared.write_text("prepared"))
                    self.assertFalse(started.exists(), f"{name}: forbidden actual child started")
                    self.assertFalse(prepared.exists(), f"{name}: forbidden preparation ran")
                    self.assertEqual([row["id"] for row in result], [cell["id"] for cell in plan])
                    self.assertTrue(all(row["status"] == "held" for row in result))
                    self.assertTrue(all(not row.get("complete", False) for row in result))
                    self.assertEqual(json.loads(ledger_path.read_text())["inflight"],
                                     original["inflight"])


if __name__ == "__main__":
    unittest.main()
