"""Hand-computed measurement oracles for the accepted Slice 130 E protocol."""

import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("slice130_soak", Path(__file__).resolve().parents[1] / "slice130_soak.py")
assert SPEC and SPEC.loader
soak = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(soak)
MIB = 1048576


def arm(mode="on", rss=lambda k: 100, reserved=lambda k: 64):
    def point(k):
        return {
            "k": k, "elapsed_s": k * 60, "rss_bytes": rss(k) * MIB,
            "vm_swap_bytes": 0, "pswpin": 10, "pswpout": 20,
            "errors": 0, "mismatches": 0, "iterations": k + 1,
            "counter_status": "ok" if mode == "on" else "no_private_pool",
            "private_reserved_bytes": reserved(k) * MIB if mode == "on" else None,
            "private_used_bytes": 32 * MIB if mode == "on" else None,
        }
    return {"sdk": "python", "pool_mode": mode, "started_monotonic_s": 0 if mode == "on" else 4000,
            "baseline": point(0), "samples": [point(k) for k in range(1, 61)],
            "exit_code": 0, "aborted": None}


class NumericalProtocolTests(unittest.TestCase):
    def test_linear_growth_has_hand_computed_slope_and_terminal_medians(self):
        result = soak.evaluate_arm(arm(rss=lambda k: 100 + k))
        self.assertEqual(result["status"], "FAIL")
        self.assertAlmostEqual(result["rss_slope_mib_per_min"], 1)
        self.assertEqual(result["rss_first_five_median_mib"], 133)
        self.assertEqual(result["rss_last_five_median_mib"], 158)
        self.assertEqual(result["rss_median_growth_mib"], 25)

    def test_baseline_and_warmup_are_excluded_from_exact_terminal_window(self):
        data = arm(rss=lambda k: 100 if k >= 31 else 100000)
        result = soak.evaluate_arm(data)
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["sample_count"], 60)
        self.assertEqual(result["terminal_count"], 30)
        self.assertEqual(result["rss_slope_mib_per_min"], 0)

    def test_actual_elapsed_times_determine_slope_with_allowed_jitter(self):
        data = arm()
        for p in data["samples"]:
            p["elapsed_s"] += 4 if p["k"] % 2 else -4
            p["rss_bytes"] = (100 + p["elapsed_s"] / 120) * MIB
        self.assertAlmostEqual(soak.evaluate_arm(data)["rss_slope_mib_per_min"], 0.5)

    def test_exact_count_indices_and_deadlines_are_required(self):
        for change in (lambda x: x["samples"].pop(),
                       lambda x: x["samples"].__setitem__(30, copy.deepcopy(x["samples"][29])),
                       lambda x: x["samples"].__setitem__(0, copy.deepcopy(x["baseline"])),
                       lambda x: x["samples"][0].update(elapsed_s=66)):
            data = arm()
            change(data)
            with self.subTest(data=data["samples"][0]):
                self.assertEqual(soak.evaluate_arm(data)["status"], "UNMEASURED")

    def test_private_reservation_range_and_growth_have_inclusive_bounds(self):
        self.assertEqual(soak.evaluate_arm(arm(reserved=lambda k: 64 if k < 60 else 96))["status"], "PASS")
        self.assertEqual(soak.evaluate_arm(arm(reserved=lambda k: 64 if k < 60 else 97))["status"], "FAIL")
        self.assertEqual(soak.evaluate_arm(arm(reserved=lambda k: 64 if k <= 55 else 80))["status"], "PASS")
        self.assertEqual(soak.evaluate_arm(arm(reserved=lambda k: 64 if k <= 55 else 81))["status"], "FAIL")

    def test_missing_error_counter_is_not_successful_zero(self):
        data = arm()
        data["samples"][40].update(counter_status="read_error", private_reserved_bytes=None)
        self.assertEqual(soak.evaluate_arm(data)["status"], "FAIL")
        data = arm("off")
        self.assertEqual(soak.evaluate_arm(data)["status"], "PASS")
        self.assertIsNone(soak.evaluate_arm(data)["reserved_range_mib"])

    def test_errors_mismatches_exit_and_swap_prevent_pass(self):
        for field, value, status in (("errors", 1, "FAIL"), ("mismatches", 1, "FAIL"),
                                    ("vm_swap_bytes", 4096, "UNMEASURED"), ("pswpin", 11, "UNMEASURED")):
            data = arm()
            data["samples"][40][field] = value
            with self.subTest(field=field):
                self.assertEqual(soak.evaluate_arm(data)["status"], status)
        data = arm()
        data["exit_code"] = 1
        self.assertEqual(soak.evaluate_arm(data)["status"], "FAIL")

    def test_pair_signed_differences_and_inclusive_control_limits(self):
        result = soak.evaluate_pair(arm(rss=lambda k: 90), arm("off"))
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["on_minus_off_median_rss_mib"], -10)
        self.assertEqual(soak.evaluate_pair(arm(rss=lambda k: 164), arm("off"))["status"], "PASS")
        self.assertEqual(soak.evaluate_pair(arm(rss=lambda k: 165), arm("off"))["status"], "FAIL")
        self.assertEqual(soak.evaluate_pair(arm(rss=lambda k: 100 + k / 4), arm("off"))["status"], "PASS")
        self.assertEqual(soak.evaluate_pair(arm(rss=lambda k: 100 + k / 3), arm("off"))["status"], "FAIL")

    def test_off_cannot_overlap_on_arm_or_use_a_different_sdk(self):
        off = arm("off")
        off["started_monotonic_s"] = 3500
        self.assertEqual(soak.evaluate_pair(arm(), off)["status"], "UNMEASURED")
        off = arm("off")
        off["sdk"] = "node"
        self.assertEqual(soak.evaluate_pair(arm(), off)["status"], "UNMEASURED")


if __name__ == "__main__":
    unittest.main()
