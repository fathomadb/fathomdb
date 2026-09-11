import json
import tempfile
import unittest
from pathlib import Path

from dev.tools import slice80_read_acceptance as subject


PASS_LOG = """
running 1 test
AC081_NUMBERS sequential_ns=199000000 concurrent_ns=79000000 sequential_searches=1600 concurrent_searches=1600 threads=8 sequential_warning=false concurrent_warning=false sequential_failure=false concurrent_failure=false ratio=2.518987
test ac_081_absolute_read_performance ... ok
test result: ok. 1 passed; 0 failed; 2 ignored; 0 measured; 20 filtered out
"""


def identity():
    return {
        "source_sha": "a" * 40,
        "binary_sha256": "b" * 64,
        "input_sha256": "c" * 64,
        "mode": "performance",
    }


def environment(**changes):
    base = {
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
    base.update(changes)
    return base


class Slice80ReadAcceptanceTests(unittest.TestCase):
    def test_parse_accepts_exact_fixture_and_identity(self):
        result = subject.parse_run_log(PASS_LOG, 0, identity())
        self.assertEqual(result["sequential_ns"], 199_000_000)
        self.assertEqual(result["concurrent_ns"], 79_000_000)
        self.assertEqual(result["threads"], 8)

    def test_parse_rejects_missing_duplicate_or_malformed_marker(self):
        for text in ("running 1 test\ntest result: ok. 1 passed", PASS_LOG + PASS_LOG):
            with self.assertRaisesRegex(ValueError, "exactly one AC081_NUMBERS"):
                subject.parse_run_log(text, 0, identity())

    def test_parse_rejects_zero_or_wrong_operation_counts(self):
        for fragment in ("sequential_searches=0", "concurrent_searches=1599", "threads=7"):
            text = PASS_LOG.replace(
                "sequential_searches=1600" if fragment.startswith("sequential") else
                "concurrent_searches=1600" if fragment.startswith("concurrent") else "threads=8",
                fragment,
            )
            with self.assertRaises(ValueError):
                subject.parse_run_log(text, 0, identity())

        zero_duration = PASS_LOG.replace("sequential_ns=199000000", "sequential_ns=0")
        with self.assertRaisesRegex(ValueError, "positive"):
            subject.parse_run_log(zero_duration, 0, identity())

    def test_parse_rejects_skipped_nonzero_exit_and_missing_identity(self):
        with self.assertRaises(ValueError):
            subject.parse_run_log(PASS_LOG.replace("1 passed", "0 passed"), 0, identity())
        with self.assertRaises(ValueError):
            subject.parse_run_log(PASS_LOG, 101, identity())
        with self.assertRaises(ValueError):
            subject.parse_run_log(PASS_LOG, 0, {**identity(), "source_sha": ""})

    def test_parse_preserves_warning_and_failure_flags(self):
        warning = PASS_LOG.replace("sequential_ns=199000000", "sequential_ns=200000000").replace(
            "sequential_warning=false", "sequential_warning=true"
        )
        self.assertTrue(subject.parse_run_log(warning, 0, identity())["sequential_warning"])
        failure = PASS_LOG.replace("concurrent_ns=79000000", "concurrent_ns=100000001").replace(
            "concurrent_failure=false", "concurrent_failure=true"
        ).replace(
            "concurrent_warning=false", "concurrent_warning=true"
        ).replace(" ... ok", " ... FAILED").replace(
            "test result: ok. 1 passed; 0 failed", "test result: FAILED. 0 passed; 1 failed"
        )
        parsed = subject.parse_run_log(failure, 101, identity())
        self.assertTrue(parsed["concurrent_failure"])

    def test_environment_accepts_qualified_start_and_end(self):
        verdict = subject.qualify_environment(environment(), environment())
        self.assertTrue(verdict["applicable"])
        self.assertEqual(verdict["reasons"], [])

    def test_environment_rejects_swap_load_memory_temperature_and_competitors(self):
        cases = [
            (environment(), environment(pswpin=11)),
            (environment(load_1m=13.0), environment()),
            (environment(available_memory_percent=24.0), environment()),
            (environment(cpu_temp_c=91.0), environment()),
            (environment(competing_processes=["cargo test"]), environment()),
        ]
        for start, end in cases:
            self.assertFalse(subject.qualify_environment(start, end)["applicable"])

    def test_environment_rejects_affinity_quota_governor_or_missing_fields(self):
        cases = [
            (environment(), environment(cpu_affinity="0-7")),
            (environment(cpu_quota="700000 100000"), environment(cpu_quota="700000 100000")),
            (environment(scaling_governors=["powersave"]), environment(scaling_governors=["powersave"])),
            (environment(), {"load_1m": 1.0}),
        ]
        for start, end in cases:
            self.assertFalse(subject.qualify_environment(start, end)["applicable"])

    def test_campaign_requires_seven_unique_matching_observations(self):
        observations = [
            {
                "label": f"R{i}",
                **identity(),
                "sequential_ns": 190_000_000,
                "concurrent_ns": 70_000_000,
                "sequential_warning": False,
                "concurrent_warning": False,
                "numeric_pass": True,
                "environment_applicable": True,
            }
            for i in range(1, 8)
        ]
        subject.validate_campaign(observations, identity())
        with self.assertRaises(ValueError):
            subject.validate_campaign(observations[:6], identity())
        with self.assertRaises(ValueError):
            subject.validate_campaign([*observations[:6], observations[0]], identity())

    def test_campaign_reports_numeric_and_environment_failure_without_raising(self):
        observations = [
            {
                "label": f"R{i}",
                **identity(),
                "sequential_ns": 190_000_000,
                "concurrent_ns": 70_000_000,
                "sequential_warning": False,
                "concurrent_warning": False,
                "numeric_pass": True,
                "environment_applicable": True,
            }
            for i in range(1, 8)
        ]
        for field, expected_status in (
            ("numeric_pass", "FAIL"),
            ("environment_applicable", "ENVIRONMENT_INVALID"),
        ):
            failed = [dict(item) for item in observations]
            failed[3][field] = False
            subject.validate_campaign(failed, identity())
            self.assertEqual(subject.summarize(failed)["status"], expected_status)

    def test_human_summary_makes_warning_visible_without_failing(self):
        observations = [
            {
                "label": f"R{i}",
                "sequential_ns": 200_000_000 if i == 1 else 190_000_000,
                "concurrent_ns": 80_000_000 if i == 1 else 70_000_000,
                "sequential_warning": i == 1,
                "concurrent_warning": i == 1,
                "numeric_pass": True,
                "environment_applicable": True,
                **identity(),
            }
            for i in range(1, 8)
        ]
        summary = subject.summarize(observations)
        self.assertEqual(summary["status"], "PASS_WITH_WARNING")
        self.assertIn("R1", subject.render_human_summary(summary))
        self.assertIn("WARNING", subject.render_human_summary(summary))

    def test_select_binary_copies_one_executable_and_records_hash(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / "perf_gates-deadbeef"
            binary.write_bytes(b"sealed")
            binary.chmod(0o755)
            build_json = root / "build.json"
            build_json.write_text(
                '{"profile":{"test":true},"target":{"name":"perf_gates"},"executable":"%s"}\n'
                % binary
            )
            destination = root / "sealed" / "perf_gates"
            record = subject.select_binary(build_json, destination)
            self.assertEqual(destination.read_bytes(), b"sealed")
            self.assertEqual(len(record["binary_sha256"]), 64)

    def test_parse_cell_consumes_raw_identity_exit_and_environment(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "R1.log"
            ident = identity()
            identity_line = " ".join(f"{key}={value}" for key, value in ident.items())
            path.write_text(
                f"SLICE80_IDENTITY {identity_line}\n"
                f"SLICE80_ENV {json.dumps({'phase': 'start', **environment()})}\n"
                f"{PASS_LOG}"
                f"SLICE80_ENV {json.dumps({'phase': 'end', **environment()})}\n"
                "SLICE80_TEST_EXIT status=0\n"
            )
            parsed = subject.parse_cell_log(path, "R1")
            self.assertEqual(parsed["label"], "R1")
            self.assertTrue(parsed["environment_applicable"])
            self.assertEqual(parsed["source_sha"], ident["source_sha"])

    def test_parse_cell_rejects_missing_or_duplicate_control_records(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "bad.log"
            path.write_text(PASS_LOG)
            with self.assertRaisesRegex(ValueError, "identity"):
                subject.parse_cell_log(path, "bad")

    def test_competing_process_scan_covers_binary_and_runner_and_excludes_ancestors(self):
        rows = """10 ac081-perf-gate /tmp/ac081-perf-gates
11 perf_gates-abcd /tmp/target/debug/deps/perf_gates-abcd --exact ac_081
12 bash bash scripts/perf-experiments/run-slice71-ac013-cell.sh . /tmp/current.log
13 bash bash scripts/perf-experiments/run-slice71-ac013-cell.sh . /tmp/other.log
14 bash bash scripts/perf-experiments/run-ac013.sh
15 cargo cargo test
16 sleep sleep 1
"""
        found = subject.scan_competing_processes(rows, {12})
        self.assertEqual([item.split()[0] for item in found], ["10", "11", "13", "14", "15"])


if __name__ == "__main__":
    unittest.main()
