"""Public preparation must retain a real terminal write when resuming its matrix."""

from copy import deepcopy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import g_closed_entry
import g_freeze
from g_campaign import run_g_plan
from test_g_serial_controller import child
from test_g_serial_matrix_entry import GSerialMatrixEntryTests


class GSerialPublicResumeTests(unittest.TestCase):
    def test_public_preflight_after_one_closed_write_uses_its_original_state(self):
        original_preflight = g_closed_entry._preflight
        def resume(root):
            frozen = deepcopy(g_freeze.validate_frozen.return_value)
            first = original_preflight(root)
            allocation = Path(first["allocation"])
            budget_path = allocation / "g-budget.json"
            budget = json.loads(budget_path.read_text())
            budget["inflight"] = None
            budget_path.write_text(json.dumps(budget))
            spec = dict(first["spec"], _max_new_writes=1)
            rows = run_g_plan(spec, root / "results", child(allocation),
                              allocation_root=allocation, reused=first["reused"])
            completed = next(row for row in rows if row["id"] == "g-w-b1-s1-engine-text-r0")
            self.assertEqual(completed["status"], "complete")
            self.assertEqual((allocation / "calls").read_text().splitlines(), [completed["id"]])
            budget = json.loads((allocation / "g-budget.json").read_text())
            self.assertEqual(budget["serial_closed_cases"][completed["id"]]["attempt"], completed["attempt"])
            g_freeze.validate_frozen.return_value = frozen
            second = original_preflight(root)
            self.assertEqual(len(second["_closed_predictions"]), 72)
            self.assertEqual((allocation / "calls").read_text().splitlines(), [completed["id"]])
            self.assertEqual(json.loads((root / "results/campaign-state.json").read_text())[-1], completed)
            return second
        with patch.object(g_closed_entry, "_preflight", side_effect=resume):
            GSerialMatrixEntryTests(
                "test_public_preflight_binds_all72_serial_predictions_without_flat_live_overreservation"
            ).test_public_preflight_binds_all72_serial_predictions_without_flat_live_overreservation()
