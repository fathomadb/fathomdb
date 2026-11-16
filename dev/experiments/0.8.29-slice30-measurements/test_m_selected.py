"""Literal finite-cell budget arithmetic, independent of future cost forecasts."""

import copy
import unittest

from m_calibration import admit_selected
from m_controller import load_plan
from test_m_calibration import LIMITS


class SelectedAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.profile = {"total_shared_ns": 14400 * 10**9,
                        "total_shared_retained_bytes": 16 * 1024**3,
                        "reserved_other_work_ns": 100 * 10**9,
                        "reserved_other_work_bytes": 1024**3}
        self.ledger = {"used_ns": 1000 * 10**9}

    def test_next_required_cell_admits_without_future_forecasts(self):
        result = admit_selected(self.profile, self.ledger, 1024**3, LIMITS, "M-R-1-1-debug")
        self.assertIsInstance(result, dict, "finite required-cell admission is missing")
        self.assertEqual(result["remaining_phase_ns"], 900 * 10**9)
        self.assertEqual(result["remaining_retained_bytes"], 2 * 1024**3)
        self.assertEqual(len(load_plan()), 46)

    def test_selected_caps_and_current_headroom_remain_binding(self):
        for change in ({"total_shared_ns": 48 * 3600 * 10**9 + 1},
                       {"total_shared_retained_bytes": 192 * 1024**3 + 1},
                       {"total_shared_ns": 2899 * 10**9}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                admit_selected(dict(self.profile, **change), self.ledger, 1024**3, LIMITS, "M-R-1-1-debug")

    def test_common_selected_totals_carry_usage_beyond_old_profile_without_mutation(self):
        profile = dict(self.profile, total_shared_ns=48 * 3600 * 10**9,
                       total_shared_retained_bytes=192 * 1024**3)
        ledger = {"used_ns": 10 * 3600 * 10**9,
                  "cell_used_ns": {"prior-failure": 1000 * 10**9},
                  "calibration_usage": {"M-CAL:M-R-1-1-warn": {"wall_ns": 900 * 10**9}},
                  "selected_cell_usage": {"M-R-1-8-warn": {"limits": copy.deepcopy(LIMITS),
                      "wall_ns": 850 * 10**9, "new_retained_bytes": 2 * 1024**3 - 100}}}
        before = copy.deepcopy((profile, ledger, LIMITS))
        result = admit_selected(profile, ledger, 20 * 1024**3, LIMITS, "M-R-1-8-warn")
        self.assertEqual(result, {"remaining_phase_ns": 50 * 10**9,
            "remaining_retained_bytes": 100, "prior_wall_ns": 850 * 10**9,
            "prior_retained_bytes": 2 * 1024**3 - 100})
        self.assertEqual((profile, ledger, LIMITS), before)
        for change in ({"reserved_other_work_ns": 38 * 3600 * 10**9},
                       {"reserved_other_work_bytes": 172 * 1024**3}):
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, "headroom"):
                admit_selected(dict(profile, **change), ledger, 20 * 1024**3, LIMITS, "M-R-1-8-warn")

    def test_original_smaller_profile_still_cannot_spend_larger_capacity(self):
        for used_ns, retained in ((12501 * 10**9, 1024**3),
                                  (1000 * 10**9, 11 * 1024**3 + 1)):
            with self.subTest(used_ns=used_ns, retained=retained), self.assertRaisesRegex(ValueError, "headroom"):
                admit_selected(self.profile, {"used_ns": used_ns}, retained, LIMITS, "M-R-1-8-warn")

    def test_common_ceiling_stays_positive_finite_and_integer(self):
        for key in ("total_shared_ns", "total_shared_retained_bytes"):
            for value in (None, False, 0, -1, 1.0):
                with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                    admit_selected(dict(self.profile, **{key: value}), self.ledger, 0, LIMITS, "M-R-1-8-warn")

    def test_failed_cell_allowance_is_cumulative_and_calibration_stays_separate(self):
        self.ledger["calibration_usage"] = {"M-CAL:M-R-1-1-warn": {"wall_ns": 900 * 10**9}}
        self.ledger["selected_cell_usage"] = {"M-R-1-1-debug": {
            "limits": LIMITS, "wall_ns": 850 * 10**9, "new_retained_bytes": 2 * 1024**3 - 100}}
        result = admit_selected(self.profile, self.ledger, 1024**3, LIMITS, "M-R-1-1-debug")
        self.assertIsInstance(result, dict, "prior selected-cell allowance was not read")
        self.assertEqual(result["remaining_phase_ns"], 50 * 10**9)
        self.assertEqual(result["remaining_retained_bytes"], 100)
        with self.assertRaises(ValueError):
            admit_selected(self.profile, self.ledger, 0, LIMITS, "made-up-cell")


if __name__ == "__main__":
    unittest.main()
