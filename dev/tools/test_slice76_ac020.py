import tempfile
import unittest
from pathlib import Path

from dev.tools import slice76_ac020


PASS_LOG = """
running 1 test
AC020_NUMBERS sequential_ms=540 concurrent_ms=121 bound_ms=101
test ac_020_reads_do_not_serialize_on_a_single_reader_connection ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 15 filtered out
"""


class Slice76Ac020Tests(unittest.TestCase):
    def test_parse_accepts_one_executed_test_and_one_marker(self):
        parsed = slice76_ac020.parse_run_log(PASS_LOG, exit_code=101)
        self.assertEqual(parsed["sequential_ms"], 540)
        self.assertEqual(parsed["concurrent_ms"], 121)
        self.assertEqual(parsed["bound_ms"], 101)
        self.assertFalse(parsed["assertion_passed"])

    def test_parse_rejects_zero_tests_or_missing_marker(self):
        with self.assertRaisesRegex(ValueError, "exactly one executed test"):
            slice76_ac020.parse_run_log("running 0 tests\ntest result: ok. 0 passed", 0)
        with self.assertRaisesRegex(ValueError, "exactly one AC020_NUMBERS"):
            slice76_ac020.parse_run_log("running 1 test\ntest result: ok. 1 passed", 0)

    def test_parse_rejects_duplicate_or_malformed_observations(self):
        with self.assertRaisesRegex(ValueError, "exactly one AC020_NUMBERS"):
            slice76_ac020.parse_run_log(PASS_LOG + PASS_LOG, 101)
        observation = {"label": "B1", "configuration": "B", "binary_sha256": "abc"}
        with self.assertRaisesRegex(ValueError, "duplicate observation label"):
            slice76_ac020.validate_observations([observation, observation], "B", "abc")
        with self.assertRaisesRegex(ValueError, "binary identity"):
            slice76_ac020.validate_observations([observation], "B", "def")

    def test_summary_uses_nearest_rank_quartiles_and_preserves_order(self):
        observations = [
            {"label": f"B{i}", "sequential_ms": value, "concurrent_ms": 100 + i}
            for i, value in enumerate([7, 1, 5, 3, 9, 11, 13], start=1)
        ]
        summary = slice76_ac020.summarize(observations)
        self.assertEqual(summary["labels"], [f"B{i}" for i in range(1, 8)])
        self.assertEqual(summary["sequential_ms"]["median"], 7)
        self.assertEqual(summary["sequential_ms"]["q1"], 3)
        self.assertEqual(summary["sequential_ms"]["q3"], 11)
        self.assertAlmostEqual(summary["ratio_of_medians"], 7 / 104)

    def test_seal_build_requires_one_binary_and_records_its_hash(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            deps = root / "release" / "deps"
            deps.mkdir(parents=True)
            binary = deps / "perf_gates-1234"
            binary.write_bytes(b"candidate")
            binary.chmod(0o755)
            destination = root / "sealed" / "perf_gates"
            record = slice76_ac020.seal_build(root, destination, "B", "source-sha")
            self.assertEqual(record["configuration"], "B")
            self.assertEqual(record["source_sha"], "source-sha")
            self.assertEqual(destination.read_bytes(), b"candidate")
            self.assertEqual(len(record["binary_sha256"]), 64)

    def test_profile_environment_delimits_the_requested_gate_phase(self):
        environment = slice76_ac020.profile_environment(
            "concurrent-after-sequential-warmup", Path("ready"), Path("go")
        )
        self.assertEqual(
            environment["FATHOMDB_SLICE76_PROFILE_ARM"],
            "concurrent-after-sequential-warmup",
        )
        self.assertEqual(environment["FATHOMDB_SLICE76_PROFILE_READY"], "ready")
        self.assertEqual(environment["FATHOMDB_SLICE76_PROFILE_GO"], "go")
        with self.assertRaisesRegex(ValueError, "unsupported profile arm"):
            slice76_ac020.profile_environment("setup", Path("ready"), Path("go"))


if __name__ == "__main__":
    unittest.main()
