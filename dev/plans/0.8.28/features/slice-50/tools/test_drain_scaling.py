"""Measurement parser and decision-rule checks."""

import unittest
from contextlib import redirect_stderr
from io import StringIO
import subprocess
from unittest.mock import patch
from drain_scaling import analyze, parse_probe, run_one


class MeasurementTests(unittest.TestCase):
    def test_parser_selects_probe_record(self):
        self.assertEqual(
            parse_probe('noise\nSLICE50_DRAIN {"n": 100}\npassed'), {"n": 100}
        )
        with self.assertRaises(ValueError):
            parse_probe("no measurement")

    def test_parser_accepts_single_thread_libtest_prefix(self):
        self.assertEqual(
            parse_probe('test slice50_drain_scaling ... SLICE50_DRAIN {"n": 100}'),
            {"n": 100},
        )

    def test_rules_use_medians_and_paired_digest(self):
        rows = [
            dict(
                label=label,
                n=n,
                repeat=r,
                mode="held",
                drain_ms=n * us / 1000,
                digest=str(n),
                load_peak=0,
            )
            for label, us in [("base", 100), ("head", 70)]
            for n in [10000, 30000, 100000]
            for r in range(2)
        ]
        result = analyze(rows)
        self.assertTrue(
            result["linear"] and result["no_regression_10k"] and result["identical"]
        )
        self.assertEqual(result["head_100k_to_10k"], 1)
        rows[-1]["digest"] = "different"
        self.assertFalse(analyze(rows)["identical"])

    def test_retry_replaces_flagged_measurement(self):
        rows = [
            dict(
                label=label,
                n=n,
                repeat=r,
                mode="held",
                drain_ms=n / 10,
                digest=str(n),
                load_peak=0,
            )
            for label in ["base", "head"]
            for n in [10000, 30000, 100000]
            for r in [0, 1]
        ]
        flagged = dict(rows[0], drain_ms=999999, load_peak=3)
        result = analyze([flagged, *rows])
        self.assertEqual(result["load_flagged"], [])
        self.assertEqual(result["head_to_base_10k"], 1)

    def test_slow_head_fails_without_threshold_rewriting(self):
        rows = [
            dict(
                label=label,
                n=n,
                repeat=r,
                mode="held",
                drain_ms=n * us / 1000,
                digest=str(n),
                load_peak=0,
            )
            for label in ["base", "head"]
            for n in [10000, 30000, 100000]
            for r in range(2)
            for us in [150 if label == "head" and n == 100000 else 100]
        ]
        self.assertFalse(analyze(rows)["linear"])

    def test_probe_failure_preserves_diagnostics(self):
        error = subprocess.CalledProcessError(
            101, ["probe"], output="probe stdout", stderr="probe failure"
        )
        output = StringIO()
        with (
            patch("drain_scaling.subprocess.run", side_effect=error),
            redirect_stderr(output),
        ):
            with self.assertRaises(subprocess.CalledProcessError):
                run_one("probe", "head", 100, 0, "overlap")
        self.assertIn("probe stdout", output.getvalue())
        self.assertIn("probe failure", output.getvalue())

    def test_missing_cell_is_not_a_pass(self):
        with self.assertRaises(ValueError):
            analyze([])


if __name__ == "__main__":
    unittest.main()
