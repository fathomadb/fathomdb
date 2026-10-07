"""Contracts for the real-engine Slice 135 baseline noise pilot."""

import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_pilot.py"
SPEC = importlib.util.spec_from_file_location("slice135_pilot", SCRIPT)
assert SPEC and SPEC.loader
pilot = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pilot)


class PilotTests(unittest.TestCase):
    def test_source_identity_checks_product_and_lock_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            checkout = Path(directory)
            (checkout / "Cargo.lock").write_bytes(b"lock")
            (checkout / "src/rust/crates/fathomdb-engine/src").mkdir(parents=True)
            (checkout / "src/rust/crates/fathomdb-engine/src/lib.rs").write_bytes(b"source")
            expected = pilot.source_tree_hash(checkout)
            self.assertEqual(expected, pilot.source_tree_hash(checkout))
            (checkout / "src/rust/crates/fathomdb-engine/src/lib.rs").write_bytes(b"changed")
            self.assertNotEqual(expected, pilot.source_tree_hash(checkout))

    def test_parse_attempts_keeps_semantic_failure_and_malformed_lines(self):
        stream = '\n'.join([
            'SLICE135_ATTEMPT {"cell":"text","valid":true,"latency_ns":7,"semantic_ok":true,"observed_checks":{"record_count":10}}',
            'SLICE135_ATTEMPT {"cell":"text","valid":false,"reason":"wrong rows","semantic_ok":false,"observed_checks":{"record_count":9}}',
            'SLICE135_ATTEMPT invalid-json',
        ])
        attempts, errors = pilot.parse_attempts(stream, ("text", "close_fresh"))
        self.assertEqual(len(attempts["text"]), 2)
        self.assertFalse(attempts["text"][1]["semantic_ok"])
        self.assertEqual(attempts["close_fresh"], [])
        self.assertEqual(len(errors), 1)

    def test_environment_invalidator_detects_swap_and_contenders(self):
        state = {
            "host": "h", "kernel": "k", "cpu": "c", "storage": "s",
            "governor": "performance", "toolchain": "rustc", "profiler": "none",
            "swap_pages": 0, "competing_jobs": [], "disk_free_bytes": 1000,
        }
        self.assertEqual(pilot.environment_invalidators(state, state, 100), [])
        changed = dict(state, swap_pages=1, competing_jobs=[{"pid": 3}])
        self.assertIn("swap activity", pilot.environment_invalidators(state, changed, 100))
        self.assertIn("competing jobs", pilot.environment_invalidators(state, changed, 100))


if __name__ == "__main__":
    unittest.main()
