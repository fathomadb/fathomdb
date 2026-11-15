"""Literal admission arithmetic; no native, model or natural workload execution."""

import copy
import unittest

from m_calibration import CALIBRATION_ID, CELL_ID, admit, selected_cell
from m_controller import load_plan


LIMITS = {"phase_ns": 900 * 10**9, "payload_ns": 800 * 10**9,
          "new_retained_bytes": 2 * 1024**3}


class CalibrationAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.profile = {"total_shared_ns": 4000 * 10**9,
                        "total_shared_retained_bytes": 10 * 1024**3,
                        "reserved_other_work_ns": 100 * 10**9,
                        "reserved_other_work_bytes": 1024**3}
        self.ledger = {"used_ns": 1000 * 10**9}

    def test_only_one_unchanged_1501_call_configuration_and_all46_primary_rows(self):
        before = load_plan()
        selected = selected_cell(CELL_ID)
        self.assertIsInstance(selected, dict, "selected calibration workload is missing")
        self.assertEqual(selected["id"], CALIBRATION_ID)
        self.assertEqual(selected["origin_id"], CELL_ID)
        self.assertEqual({k: v for k, v in selected.items() if k not in ("id", "origin_id")},
                         {k: v for k, v in before[2].items() if k != "id"})
        self.assertEqual((selected["first"], selected["warmups"], selected["timed"]), (1, 500, 1000))
        for other in ("M-R-1-1-debug", "M-R-2-1-warn", "M0-text"):
            with self.assertRaisesRegex(ValueError, "selected"):
                selected_cell(other)
        self.assertEqual(load_plan(), before)
        self.assertEqual(len(before), 46)

    def test_positive_twofold_headroom_and_external_reservations(self):
        result = admit(self.profile, self.ledger, 1024**3, LIMITS)
        self.assertIsInstance(result, dict, "bounded calibration admission is missing")
        self.assertEqual(result["remaining_phase_ns"], 900 * 10**9)
        self.assertEqual(result["remaining_retained_bytes"], 2 * 1024**3)
        for name, changed in (("time", {"total_shared_ns": 2899 * 10**9}),
                              ("storage", {"total_shared_retained_bytes": 6 * 1024**3-1})):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "headroom"):
                admit(dict(self.profile, **changed), self.ledger, 1024**3, LIMITS)
        for key in LIMITS:
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "allowance"):
                admit(self.profile, self.ledger, 0, dict(LIMITS, **{key: 0}))

    def test_prior_failed_wall_and_retained_growth_survive_new_roots_and_sources(self):
        self.ledger["calibration_usage"] = {CALIBRATION_ID: {
            "limits": copy.deepcopy(LIMITS), "wall_ns": 850 * 10**9,
            "new_retained_bytes": 2 * 1024**3-100}}
        result = admit(self.profile, self.ledger, 1024**3, LIMITS)
        self.assertIsInstance(result, dict, "prior calibration usage was not read")
        self.assertEqual(result["remaining_phase_ns"], 50 * 10**9)
        self.assertEqual(result["remaining_retained_bytes"], 100)
        self.ledger["calibration_usage"][CALIBRATION_ID]["wall_ns"] = 881 * 10**9
        with self.assertRaisesRegex(ValueError, "remaining"):
            admit(self.profile, self.ledger, 0, LIMITS)
        with self.assertRaisesRegex(ValueError, "changed"):
            admit(self.profile, self.ledger, 0, dict(LIMITS, phase_ns=899 * 10**9))


if __name__ == "__main__":
    unittest.main()
