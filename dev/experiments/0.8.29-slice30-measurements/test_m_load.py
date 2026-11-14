"""Controlled host-load boundaries preserve real child results and descriptive noise facts."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from m_controller import COMPONENTS, load_plan, run_budgeted_table


class CellLoadTests(unittest.TestCase):
    """A runtime observation above2.5 marks noise without dropping actual results."""

    def test_load_boundary_is_descriptive_for_completed_actual_children(self):
        for peak, noisy in ((2.5, False), (2.5001, True)):
            with self.subTest(peak=peak), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                ledger = root / "g-budget.json"
                ledger.write_text(json.dumps({"schema": "slice30-gd-budget/v1", "used_ns": 0,
                                              "inflight": None, "cell_used_ns": {}}))
                profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
                    "ledger": str(ledger), "whole_cell_ns": 900 * 10**9,
                    "total_shared_ns": 60 * 10**9, "total_shared_retained_bytes": 1024**2}
                plan = load_plan()[:1]
                forecasts = {plan[0]["id"]: {"components_ns": dict.fromkeys(COMPONENTS, 0),
                                            "new_retained_bytes": 0}}
                child = ("import json,pathlib,sys,time;time.sleep(.03);"
                         "(pathlib.Path(sys.argv[2])/'result.json').write_text("
                         "json.dumps({'complete':True,'literal':'kept'}))")
                observations = iter((2.0, peak))
                with patch("os.getloadavg", side_effect=lambda: (next(observations, peak), 0.0, 0.0)):
                    result = run_budgeted_table(root / "results", [sys.executable, "-c", child],
                        profile=profile, allocation=root, forecasts=forecasts, plan=plan)
                self.assertTrue(result[0]["complete"])
                self.assertEqual(result[0].get("load", {}).get("noisy"), noisy,
                                 "actual runtime load/noise fact was absent or lost its strict boundary")
                self.assertEqual(result[0]["load"]["max_load"], peak)
                saved = json.loads((Path(result[0]["attempt"]) / "result.json").read_text())
                self.assertEqual(saved, {"complete": True, "literal": "kept"})
