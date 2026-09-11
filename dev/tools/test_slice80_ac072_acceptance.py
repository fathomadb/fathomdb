import json
import unittest

from dev.tools import slice80_ac072_acceptance as subject


def environment(**changes):
    value = {
        "load_1m": 1.0,
        "online_cpus": 24,
        "available_memory_percent": 70.0,
        "pswpin": 10,
        "pswpout": 20,
        "cpu_temp_c": 55.0,
        "thermal_signal": "k10temp:Tctl",
        "competing_processes": [],
        "cpu_affinity": "0-23",
        "cpu_quota": "max 100000",
        "scaling_governors": ["performance"],
        "scaling_frequencies_khz": [3500000],
    }
    value.update(changes)
    return value


def log(n=10_000, purpose="acceptance", status=0, samples=None):
    samples = samples or ",".join(["70000"] * 1000)
    return "\n".join(
        [
            "SLICE80_AC072_IDENTITY source_sha=" + "a" * 40
            + " binary_sha256=" + "b" * 64
            + " input_sha256=" + "c" * 64
            + " runner_sha256=" + "d" * 64
            + " scanner_sha256=" + "e" * 64
            + f" mode=performance purpose={purpose} selector=ac_013_vector_retrieval_latency corpus_n={n} vector_dim=384 requested_samples=1000 compiled_samples=1000 treatment=warm",
            f"SLICE80_AC072_INVOKE AGENT_LONG=1 AC013_CORPUS_N={n} AC013_VECTOR_DIM=384 AC013_SAMPLES=1000 AC013_SCALE_TREATMENT=warm",
            "SLICE71_ENV " + json.dumps({"phase": "start", **environment()}),
            f"AC013_NUMBERS n={n} samples=1000 seed_ms=1 p50_ms=70 p99_ms=70",
            f"AC013_TREATMENT_RECORD treatment=warm n={n} seed_write_ms=1 embedding_ms=not_separately_observable projection_drain_ms=1 accepted_writes={n} vector_rows_after_drain={n} drain_outcome=ok samples_us={samples} result_counts=not_retained_per_query",
            "test ac_013_vector_retrieval_latency ... ok",
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out",
            "SLICE71_ENV " + json.dumps({"phase": "end", **environment()}),
            f"SLICE80_AC072_TEST_EXIT status={status}",
        ]
    ) + "\n"


class Slice80Ac072AcceptanceTests(unittest.TestCase):
    def test_accepts_complete_qualified_warm_cell(self):
        cell = subject.parse_cell_text(log(), "R1", "acceptance")
        self.assertTrue(cell["numeric_pass"])
        self.assertTrue(cell["environment_applicable"])
        self.assertEqual(cell["sample_count"], 1000)

    def test_accepts_the_rust_harness_progress_prefix_on_numbers(self):
        prefixed = log().replace(
            "AC013_NUMBERS n=10000", "test ac_013_vector_retrieval_latency ... AC013_NUMBERS n=10000"
        )
        cell = subject.parse_cell_text(prefixed, "R1", "acceptance")
        self.assertTrue(cell["numeric_pass"])

    def test_accepts_the_rust_harness_late_ok_after_stderr_records(self):
        split = log().replace(
            "AC013_NUMBERS n=10000", "test ac_013_vector_retrieval_latency ... AC013_NUMBERS n=10000"
        ).replace("test ac_013_vector_retrieval_latency ... ok\n", "ok\n")
        cell = subject.parse_cell_text(split, "R1", "acceptance")
        self.assertTrue(cell["numeric_pass"])

    def test_smoke_has_own_size_and_cannot_satisfy_acceptance(self):
        smoke = subject.parse_cell_text(log(n=10, purpose="smoke"), "smoke", "smoke")
        self.assertEqual(smoke["corpus_n"], 10)
        with self.assertRaisesRegex(ValueError, "purpose"):
            subject.parse_cell_text(log(n=10, purpose="smoke"), "R1", "acceptance")

    def test_rejects_missing_controls_duplicate_markers_and_bad_samples(self):
        for broken in (
            log().replace("AC013_VECTOR_DIM=384 ", ""),
            log() + "AC013_NUMBERS n=10000 samples=1000 seed_ms=1 p50_ms=70 p99_ms=70\n",
            log(samples="70000," * 999),
            log().replace("1 passed; 0 failed; 0 ignored", "0 passed; 0 failed; 1 ignored"),
        ):
            with self.assertRaises(ValueError):
                subject.parse_cell_text(broken, "R1", "acceptance")

    def test_strict_numeric_failure_preserves_environment_verdict(self):
        failed = log(samples=",".join(["81000"] * 1000)).replace("p50_ms=70 p99_ms=70", "p50_ms=81 p99_ms=81")
        failed = failed.replace("... ok", "... FAILED").replace(
            "test result: ok. 1 passed; 0 failed", "test result: FAILED. 0 passed; 1 failed"
        ).replace("SLICE80_AC072_TEST_EXIT status=0", "SLICE80_AC072_TEST_EXIT status=101")
        cell = subject.parse_cell_text(failed, "R1", "acceptance")
        self.assertFalse(cell["numeric_pass"])
        self.assertTrue(cell["environment_applicable"])
        self.assertEqual(subject.cell_status(cell), "FAIL")

    def test_full_precision_boundary_failure_is_not_misread_from_truncated_milliseconds(self):
        samples = ",".join(["80000"] * 989 + ["300000"] * 11)
        failed = log(samples=samples).replace("p50_ms=70 p99_ms=70", "p50_ms=80 p99_ms=300")
        failed = failed.replace("... ok", "... FAILED").replace(
            "test result: ok. 1 passed; 0 failed", "test result: FAILED. 0 passed; 1 failed"
        ).replace("SLICE80_AC072_TEST_EXIT status=0", "SLICE80_AC072_TEST_EXIT status=101")
        for text in (
            failed,
            failed.replace('"pswpout": 20', '"pswpout": 21', 1),
        ):
            cell = subject.parse_cell_text(text, "R1", "acceptance")
            self.assertFalse(cell["numeric_pass"])
            self.assertTrue(cell["environment_applicable"])
            self.assertEqual(subject.cell_status(cell), "FAIL")

    def test_numeric_verdict_and_machine_wide_swap_diagnostic_remain_distinct(self):
        diagnostic = log().replace('"pswpin": 10', '"pswpin": 11', 1)
        cell = subject.parse_cell_text(diagnostic, "R1", "acceptance")
        self.assertTrue(cell["numeric_pass"])
        self.assertTrue(cell["environment_applicable"])
        self.assertEqual(
            cell["environment_diagnostics"],
            [{"kind": "machine_wide_swap", "pswpin_delta": -1, "pswpout_delta": 0}],
        )
        self.assertEqual(subject.cell_status(cell), "PASS")

    def test_campaign_requires_three_unique_applicable_numeric_passes(self):
        cell = subject.parse_cell_text(log(), "R1", "acceptance")
        with self.assertRaisesRegex(ValueError, "exactly three"):
            subject.summarize_campaign([cell, cell])
        failed = {**cell, "label": "R2", "numeric_pass": False}
        with self.assertRaisesRegex(ValueError, "not passing"):
            subject.summarize_campaign([cell, failed, {**cell, "label": "R3"}])
