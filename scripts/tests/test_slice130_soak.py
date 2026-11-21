"""Hand-computed measurement oracles for the accepted Slice 130 E protocol."""

import copy
import importlib.util
from pathlib import Path
import unittest
import tracemalloc
import os
import sys
import tempfile
import threading
import selectors
import subprocess
import shutil
import time
from unittest.mock import patch

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

    def test_missing_exit_and_deadline_are_unmeasured_while_actual_failure_stays_fail(self):
        data = arm()
        data["exit_code"] = None
        self.assertEqual(soak.evaluate_arm(data)["status"], "UNMEASURED")
        data = arm()
        data.update(aborted="sample 1 deadline exceeded", abort_kind="deadline")
        self.assertEqual(soak.evaluate_arm(data)["status"], "UNMEASURED")
        data.update(aborted="actual forward failed", abort_kind="operation_error")
        self.assertEqual(soak.evaluate_arm(data)["status"], "FAIL")

    def test_observed_operation_failures_survive_incomplete_or_invalid_measurements(self):
        for field in ("errors", "mismatches"):
            for short in (False, True):
                data = arm()
                data["samples"][0][field] = 1
                if short:
                    data["samples"] = data["samples"][:1]
                else:
                    del data["samples"][0]["rss_bytes"]
                with self.subTest(field=field, short=short):
                    result = soak.evaluate_arm(data)
                    self.assertEqual(result["status"], "FAIL")
                    self.assertIn("operation errors or output mismatches", result["reasons"])
                    self.assertTrue(any("missing" in reason or "60 scheduled" in reason for reason in result["reasons"]))

    def test_named_counter_failure_survives_a_short_series_and_retains_gap_reason(self):
        for mode in ("on", "off"):
            data = arm(mode)
            data["samples"] = data["samples"][:1]
            data["samples"][0].update(counter_status="read_error", private_reserved_bytes=None,
                                      raw_observation={"status": "read_error", "context_id": "18446744073709551615"})
            with self.subTest(mode=mode):
                result = soak.evaluate_arm(data)
                self.assertEqual(result["status"], "FAIL")
                self.assertTrue(any("counter" in reason for reason in result["reasons"]))
                self.assertIn("separate baseline and exactly 60 scheduled samples required", result["reasons"])

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


class NodeDefaultProfileTests(unittest.TestCase):
    def profile_arm(self, mode="on", slope=0):
        value = arm(mode, rss=lambda k: 100 + slope * k / 6)
        value.update(sdk="node", profile="node-default-10m")
        for point in [value["baseline"], *value["samples"]]:
            # Integer byte counters on a dyadic clock avoid a floating-point
            # fixture exceeding the exact inclusive boundary by one ULP.
            ticks = (320 * point["k"] + 14) // 15
            point["elapsed_s"] = ticks * 15 / 32
            point["rss_bytes"] = 100 * MIB + slope * ticks * (MIB // 128)
            point["node_heap"] = {key: 8 * MIB for key in ("rss", "heapUsed", "heapTotal", "external", "arrayBuffers")}
            point["node_v8"] = {"heap": {key: 8 * MIB for key in ("used_heap_size", "total_heap_size", "heap_size_limit", "external_memory")}, "spaces": [{"space_name": "new_space", **{key: MIB for key in ("space_size", "space_used_size", "space_available_size", "physical_space_size")}}]}
        return value

    def test_ten_minute_profile_uses_real_seconds_and_original_bounds(self):
        result = soak.evaluate_arm(self.profile_arm(slope=0.5))
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["terminal_count"], 30)
        self.assertAlmostEqual(result["rss_slope_mib_per_min"], 0.5)
        self.assertEqual(soak.evaluate_arm(self.profile_arm(slope=1))["status"], "FAIL")

    def test_failed_absolute_arm_still_reports_signed_matched_diagnostic_deltas(self):
        result = soak.evaluate_pair(self.profile_arm(slope=1), self.profile_arm("off", slope=0))
        self.assertEqual(result["status"], "FAIL")
        self.assertAlmostEqual(result["on_minus_off_slope_mib_per_min"], 1)
        self.assertAlmostEqual(result["on_minus_off_median_rss_mib"], (960 + 982) / 2 / 128)
        self.assertEqual(result["on"]["status"], "FAIL")


    def test_default_profile_requires_passive_js_and_v8_measurements(self):
        for field in ("node_heap", "node_v8"):
            data = self.profile_arm()
            del data["samples"][20][field]
            self.assertEqual(soak.evaluate_arm(data)["status"], "UNMEASURED")

    def test_operation_failure_does_not_gain_diagnostic_pair_metrics(self):
        on, off = self.profile_arm(), self.profile_arm("off")
        off["samples"][20]["errors"] = 1
        result = soak.evaluate_pair(on, off)
        self.assertEqual(result["status"], "FAIL")
        self.assertNotIn("on_minus_off_slope_mib_per_min", result)



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

    def test_named_counter_failures_stop_immediately_and_retain_raw_observation(self):
        for mode in ("on", "off"):
            for status in ("read_error", "unavailable"):
                for failed_k in (0, 1):
                    now = [0.0]
                    calls = [0]
                    observations = [0]
                    events = []
                    def iteration():
                        now[0] += 60
                        calls[0] += 1
                        return {"latency_ms": {"embed": 1}, "mismatches": 0}
                    def snapshot():
                        k = observations[0]
                        observations[0] += 1
                        point = arm(mode)["baseline"]
                        if k == failed_k:
                            point.update(counter_status=status, private_used_bytes=None, private_reserved_bytes=None,
                                         raw_observation={"status": status, "context_id": "18446744073709551615"})
                        return point
                    data = soak.collect_arm("python", mode, iteration, snapshot, events.append, clock=lambda: now[0])
                    with self.subTest(mode=mode, status=status, failed_k=failed_k):
                        self.assertEqual(calls[0], failed_k + 1)  # warm-up, then at most one measured iteration
                        self.assertEqual(len(data["samples"]), failed_k)
                        self.assertEqual(data["abort_kind"], "counter_error")
                        self.assertIn(status, data["aborted"])
                        failed = data["baseline"] if failed_k == 0 else data["samples"][0]
                        self.assertEqual(failed["raw_observation"], {"status": status, "context_id": "18446744073709551615"})
                        self.assertTrue(any(e.get("point", {}).get("counter_status") == status for e in events))

    def test_success_status_with_invalid_private_counter_stops_but_named_off_is_valid(self):
        for mode in ("on", "off"):
            now = [0.0]
            def iteration():
                now[0] += 60
                return {"latency_ms": {"embed": 1}, "mismatches": 0}
            point = arm(mode)["baseline"]
            if mode == "on":
                point["private_used_bytes"] = None
            data = soak.collect_arm("python", mode, iteration, lambda: dict(point), lambda e: None, clock=lambda: now[0])
            with self.subTest(mode=mode):
                if mode == "on":
                    self.assertEqual(data["abort_kind"], "counter_error")
                    self.assertEqual(len(data["samples"]), 0)
                else:
                    self.assertIsNone(data["aborted"])
                    self.assertEqual(len(data["samples"]), 60)

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


class WorkerCleanupTests(unittest.TestCase):
    def test_repeated_reap_never_signals_a_reused_process_group(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryFile() as stderr:
            worker = soak.Worker([sys.executable, "-c", "import sys; sys.stdin.read()"], directory, os.environ.copy(), stderr, threading.Event())
            first = worker.reap()
            with patch.object(soak.os, "killpg", side_effect=AssertionError("a reaped process group must never be signaled again")):
                self.assertEqual(worker.reap(), first)


@unittest.skipUnless(sys.platform == "linux" and shutil.which("cc"), "Linux procfs and a C compiler required")
class OwnedExitTransitionTests(unittest.TestCase):
    def test_pending_reap_after_main_thread_exit_is_bounded_and_identity_checked(self):
        root = Path(__file__).resolve().parents[2]
        spec = importlib.util.spec_from_file_location("profile_guard", root / "dev/tools/slice50_orin_profile.py")
        guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(guard)
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "exit-transition"
            compiled = subprocess.run([shutil.which("cc"), "-std=c11", "-O2", "-pthread", str(root / "dev/experiments/slice50-release/proc_exit_transition.c"), "-o", str(binary)], capture_output=True, text=True)
            if compiled.returncode:
                sys.stdout.write(compiled.stdout)
                sys.stderr.write(compiled.stderr)
                self.fail("CPU transition fixture did not compile")
            child = subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, start_new_session=True)
            selector = selectors.DefaultSelector()
            selector.register(child.stdout, selectors.EVENT_READ)
            release_thread = None
            def line():
                self.assertTrue(selector.select(5), "bounded child handshake")
                return child.stdout.readline().decode().strip()
            try:
                self.assertEqual(line(), "READY")
                birth = guard.proc(child.pid)["start_ticks"]
                child._slice50_birth = birth
                with patch.object(guard, "actors", return_value=[]):
                    self.assertFalse(guard.confirmed_owned_exit(child, birth, [child.pid]))
                    self.assertFalse(guard.confirmed_owned_exit(child, birth, [child.pid, os.getpid()]))
                    child.stdin.write(b"G")
                    child.stdin.flush()
                    self.assertEqual(line(), "EXE_GONE")
                    with self.assertRaises(FileNotFoundError):
                        Path(f"/proc/{child.pid}/exe").resolve(strict=True)
                    pending = threading.Event()
                    original_poll = child.poll
                    def poll():
                        code = original_poll()
                        if code is None:
                            pending.set()
                        return code
                    child.poll = poll
                    def release():
                        if pending.wait(5):
                            child.stdin.write(b"E")
                            child.stdin.flush()
                    release_thread = threading.Thread(target=release)
                    release_thread.start()
                    self.assertTrue(guard.confirmed_owned_exit(child, birth, []))
                    release_thread.join(5)
                    self.assertFalse(release_thread.is_alive())
                    self.assertEqual(child.wait(timeout=5), 0)
                    self.assertFalse(guard.confirmed_owned_exit(child, None, []))
                    self.assertFalse(guard.confirmed_owned_exit(child, birth, [os.getpid()]))
                    self.assertFalse(guard.confirmed_owned_exit(child, birth, [], deadline=time.monotonic() - 1))
                with patch.object(guard, "actors", return_value=[os.getpid()]):
                    self.assertFalse(guard.confirmed_owned_exit(child, birth, []))
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait(timeout=5)
                if release_thread:
                    release_thread.join(5)
                selector.close()
                child.stdin.close()
                child.stdout.close()


class RawObservationBindingTests(unittest.TestCase):
    def test_success_label_cannot_hide_changed_raw_pool_or_unavailable_counters(self):
        point = arm()["baseline"]
        point["raw_observation"] = {
            "schema": "fathomdb.cuda-pool-observation/v1", "status": "read_error",
            "pid": 123, "device_ordinal": 0, "uuid": "actual-device",
            "context_id": "18446744073709551615", "pool_id": "actual-pool",
            "live_models_before": "2", "live_models_after": "2",
            "used_current_bytes": None, "reserved_current_bytes": None,
        }
        self.assertIsNotNone(soak._counter_failure(point, "on"),
                             "raw read_error cannot be relabeled as successful checked counters")

    def test_success_label_cannot_hide_changed_raw_ownership(self):
        point = arm()["baseline"]
        point["raw_observation"] = {
            "schema": "fathomdb.cuda-pool-observation/v1", "status": "private",
            "pid": 123, "device_ordinal": 0, "uuid": "actual-device",
            "context_id": "18446744073709551615", "pool_id": "actual-pool",
            "live_models_before": "2", "live_models_after": "1",
            "used_current_bytes": str(point["private_used_bytes"]),
            "reserved_current_bytes": str(point["private_reserved_bytes"]),
        }
        self.assertIsNotNone(soak._counter_failure(point, "on"),
                             "a live-owner transition invalidates the supposedly checked snapshot")


if __name__ == "__main__":
    unittest.main()
