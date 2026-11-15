"""Hand-computed measurement oracles for the accepted Slice 130 E protocol."""

import copy
import importlib.util
from pathlib import Path
import unittest
import tracemalloc

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


class LinuxMemoryTests(unittest.TestCase):
    STATUS = "VmRSS: 10 kB\nRssAnon: 6 kB\nRssFile: 3 kB\nRssShmem: 1 kB\nVmSwap: 0 kB\n"
    MEMINFO = "MemTotal: 100 kB\nMemAvailable: 40 kB\n"
    VMSTAT = "pswpin 7\npswpout 9\n"

    def test_procfs_kibibytes_convert_to_bytes_and_swap_counts_stay_counts(self):
        result = soak.parse_linux_memory(self.STATUS, self.MEMINFO, self.VMSTAT)
        self.assertEqual(result.get("rss_bytes"), 10240)
        self.assertEqual(result.get("rss_anon_bytes"), 6144)
        self.assertEqual(result.get("rss_file_bytes"), 3072)
        self.assertEqual(result.get("rss_shmem_bytes"), 1024)
        self.assertEqual(result.get("vm_swap_bytes"), 0)
        self.assertEqual(result.get("mem_available_bytes"), 40960)
        self.assertEqual((result.get("pswpin"), result.get("pswpout")), (7, 9))

    def test_missing_required_counter_is_unavailable_not_zero(self):
        with self.assertRaisesRegex(ValueError, "VmSwap"):
            soak.parse_linux_memory(self.STATUS.replace("VmSwap: 0 kB\n", ""), self.MEMINFO, self.VMSTAT)


class SamplerProtocolTests(unittest.TestCase):
    def test_sampler_warms_once_then_records_separate_baseline_and_exact_sixty_samples(self):
        now = [0.0]
        calls = []
        events = []
        def iteration():
            now[0] += 60
            calls.append(now[0])
            return {"latency_ms": {"embed": 1, "batch32": 2, "rerank": 3}, "mismatches": 0,
                    "outputs": {"embed_sha": "same-input-same-device"}}
        def snapshot():
            return {"rss_bytes": 100 * MIB, "raw_observation": {"context_id": "18446744073709551615"}}
        data = soak.collect_arm("python", "on", iteration, snapshot, events.append, clock=lambda: now[0])
        self.assertEqual(len(data["samples"]), 60)
        self.assertEqual(data["baseline"]["elapsed_s"], 0)
        self.assertEqual(data["baseline"]["k"], 0)
        self.assertEqual([p["elapsed_s"] for p in data["samples"]], list(range(60, 3601, 60)))
        self.assertEqual(data["samples"][-1]["iterations"], 60)
        self.assertEqual(len(calls), 61)  # one complete warm-up is outside the measured loop
        self.assertEqual(data["warmup_duration_s"], 60)
        self.assertEqual(len([e for e in events if e["event"] == "iteration"]), 60)
        self.assertEqual(data["samples"][0]["raw_observation"]["context_id"], "18446744073709551615")
        self.assertIsNone(data["exit_code"])  # actual exit must come from the reaping supervisor

    def test_late_sample_stops_instead_of_fabricating_catchup_rows(self):
        now = [0.0]
        def iteration():
            now[0] += 130
            return {"latency_ms": {"embed": 1}, "mismatches": 0}
        data = soak.collect_arm("python", "on", iteration, lambda: {}, lambda e: None, clock=lambda: now[0])
        self.assertEqual(len(data["samples"]), 1)
        self.assertEqual(data["samples"][0]["k"], 1)
        self.assertEqual(data["samples"][0]["elapsed_s"], 130)
        self.assertIn("deadline", data["aborted"])

    def test_failure_stops_and_retains_actual_error(self):
        now = [0.0]
        calls = [0]
        events = []
        def iteration():
            calls[0] += 1
            now[0] += 1
            if calls[0] > 1:
                raise RuntimeError("actual forward failed")
            return {"latency_ms": {"embed": 1}, "mismatches": 0}
        data = soak.collect_arm("python", "on", iteration, lambda: {}, events.append, clock=lambda: now[0])
        self.assertIn("actual forward failed", data["aborted"])
        self.assertEqual(calls[0], 2)
        self.assertTrue(any(e["event"] == "error" for e in events))

    def test_primary_sampler_has_bounded_allocation_without_deliberate_churn(self):
        now = [0.0]
        def iteration():
            now[0] += 60
            return {"latency_ms": {"embed": 1}, "mismatches": 0}
        tracemalloc.start()
        try:
            data = soak.collect_arm("python", "on", iteration, lambda: {}, lambda e: None, clock=lambda: now[0])
            _, peak = tracemalloc.get_traced_memory()
        finally:
            tracemalloc.stop()
        self.assertEqual(len(data["samples"]), 60)
        self.assertLess(peak, 2 * MIB)  # the historical 200000-dict sampling churn exceeds this


if __name__ == "__main__":
    unittest.main()
