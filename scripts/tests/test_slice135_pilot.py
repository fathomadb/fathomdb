"""Contracts for the real-engine Slice 135 baseline noise pilot."""

import importlib.util
import hashlib
import json
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

    def test_reused_binary_requires_exact_source_runner_lock_and_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            previous = root / "previous"
            previous.mkdir()
            (previous / "build").mkdir()
            binary = b"real built binary bytes"
            lock = b"resolved lock bytes"
            (previous / "slice135_pilot_workload").write_bytes(binary)
            (previous / "build/Cargo.lock").write_bytes(lock)
            source = {"source_sha": "a" * 40, "source_tree_sha256": "b" * 64,
                      "cargo_lock_sha256": "c" * 64}
            (previous / "source.json").write_text(json.dumps(source))
            runner_hash = "d" * 64
            (previous / "runner.bundle").write_bytes(b"runner")
            protocol = {
                "source_sha": source["source_sha"],
                "runner_sha256": hashlib.sha256(b"runner").hexdigest(),
                "artifact_sha256": {
                    "slice135_pilot_workload": hashlib.sha256(binary).hexdigest(),
                    "build/Cargo.lock": hashlib.sha256(lock).hexdigest(),
                },
            }
            (previous / "pilot-protocol.json").write_text(json.dumps(protocol))
            (previous / "attempt.json").write_text(json.dumps({"status": "SMOKE_ONLY"}))
            target = root / "next"
            target.mkdir()
            expected = protocol["artifact_sha256"]["slice135_pilot_workload"]
            with self.assertRaisesRegex(ValueError, "binary hash"):
                pilot.reuse_binary(previous, target, source, protocol["runner_sha256"], "0" * 64)
            with self.assertRaisesRegex(ValueError, "runner"):
                pilot.reuse_binary(previous, target, source, runner_hash, expected)
            with self.assertRaisesRegex(ValueError, "source"):
                pilot.reuse_binary(previous, target, dict(source, source_sha="f" * 40),
                                   protocol["runner_sha256"], expected)
            (previous / "build/Cargo.lock").write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "lock"):
                pilot.reuse_binary(previous, target, source, protocol["runner_sha256"], expected)
            (previous / "build/Cargo.lock").write_bytes(lock)
            copied = pilot.reuse_binary(previous, target, source,
                                       protocol["runner_sha256"], expected)
            self.assertEqual(copied.read_bytes(), binary)


if __name__ == "__main__":
    unittest.main()
