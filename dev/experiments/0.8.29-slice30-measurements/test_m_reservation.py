"""Retained common bytes cannot be silently assigned to a prepared M cell."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from m_controller import COMPONENTS, load_plan, run_budgeted_table


class SharedReservationTests(unittest.TestCase):
    """Admission funds complete common storage before any harmless child starts."""

    def test_common_storage_reservation_refuses_before_actual_dispatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ledger = root / "g-budget.json"
            ledger.write_text(json.dumps({"schema": "slice30-gd-budget/v1", "used_ns": 0,
                                          "inflight": None, "cell_used_ns": {}}))
            marker = root / "started"
            child = (f"import pathlib,sys;pathlib.Path({str(marker)!r}).write_text('started');"
                     "(pathlib.Path(sys.argv[2])/'result.json').write_text('{\"complete\":true}')")
            plan = load_plan()[:1]
            profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
                       "ledger": str(ledger), "total_shared_ns": 60 * 10**9,
                       "total_shared_retained_bytes": 1024**2, "whole_cell_ns": 900 * 10**9}
            forecast = {plan[0]["id"]: {"components_ns": dict.fromkeys(COMPONENTS, 0),
                                      "new_retained_bytes": 0}}
            result = run_budgeted_table(root / "results", [sys.executable, "-c", child],
                profile=profile, allocation=root, forecasts=forecast, plan=plan,
                common_reserved_bytes=1024**2)
            self.assertFalse(marker.exists(), "reserved common bytes were consumed by an actual cell")
            self.assertEqual(result[0]["status"], "held")


if __name__ == "__main__":
    unittest.main()
