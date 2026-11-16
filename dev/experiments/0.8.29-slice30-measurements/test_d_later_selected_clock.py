"""Explicit later clock selection keeps fixed workloads and requires actual lower evidence."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from d_controller import assess_stage
from d_driver import run_supervised_stage
from test_d_controller import literal_records, SPEC

SECOND, MIB = 10**9, 1024**2


class DLaterClockTests(unittest.TestCase):
    def later(self):
        records = literal_records(10000, 100000)
        for row in records.values():
            row.update(attempt_id="literal-original", elapsed_ns=(70 if row["n"] == 10000 else 700) * SECOND,
                       peak_bytes=(32 if row["n"] == 10000 else 256) * MIB)
        budget = {"used_ns": 100 * SECOND, "shared_ns": 30000 * SECOND,
            "retained_bytes": 2 * 1024**3, "storage_limit": 16 * 1024**3, "inflight": None,
            "cell_used_ns": {}, "cell_limits_ns": {"300000": 4500 * SECOND}}
        review = {"verdict": "ACCEPT", "reviewer": "literal-review", "subject": "later-bounded-selection",
            "n": 300000, "mode": "held",
            "checkpoint": {"used_ns": 100 * SECOND, "retained_bytes": 2 * 1024**3},
            "prefix_attempts": {key: row["attempt_id"] for key, row in records.items()},
            "bounds": {layer: {"time_ns": 4200 * SECOND, "active_bytes": 1920 * MIB} for layer in ("C", "T")}}
        return records, budget, review

    def test_later_explicit4500s_bound_uses_current_lower_evidence_without_changing_defaults(self):
        records, budget, _ = self.later()
        self.assertEqual(assess_stage(300000, "held", records, budget)["status"], "admit",
                         "later explicit bounded selection still inherits compiled900s clock")
        original = dict(budget, cell_limits_ns={})
        self.assertEqual(assess_stage(300000, "held", records, original)["status"], "not_run_cap_feasibility")
        del records["d-c-held-n100000-r1"]
        self.assertEqual(assess_stage(300000, "held", records, budget)["status"], "not_run_lower_evidence")

    def test_later_selection_reaches_actual_owned_child_config_and_deadline(self):
        records, budget, review = self.later()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            code = "import pathlib,sys;pathlib.Path(sys.argv[2],'result.json').write_text('{\"complete\":false,\"unit_diagnostic\":true}')"
            run_supervised_stage(SPEC, records, budget, root, [sys.executable, "-c", code],
                                 initial=False, review=review)
            configs = list(root.glob("d-*-held-n300000-*/attempt-0001/cell.json"))
            self.assertEqual(len(configs), 4, "selected later clocks never reached owned dispatch")
            for path in configs:
                cell = json.loads(path.read_text())
                self.assertEqual(cell["cell_ns"], 4500 * SECOND)
                self.assertGreater(cell["remaining_cell_ms"], 900000)
                self.assertLessEqual(cell["remaining_cell_ms"], 4500000)


if __name__ == "__main__":
    unittest.main()
