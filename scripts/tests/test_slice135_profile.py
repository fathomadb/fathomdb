"""Bound sampled profile classification for the Slice 135 real workload."""

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_profile.py"
SPEC = importlib.util.spec_from_file_location("slice135_profile", SCRIPT)
assert SPEC and SPEC.loader
profile = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(profile)


class ProfileTests(unittest.TestCase):
    def test_mixed_engine_frames_exclude_harness_only_stacks(self):
        self.assertTrue(profile.operation_frame(
            "mixed_sequence", "#0 sqlite3_step\n#1 fathomdb_engine::erasure::Engine::erase_source",
        ))
        self.assertFalse(profile.operation_frame(
            "mixed_sequence", "#0 sqlite3_step\n#1 slice135_pilot_workload::mixed_sequence",
        ))
        self.assertTrue(profile.operation_frame(
            "text", "#0 sqlite3_step\n#1 fathomdb_engine::search_api::Engine::search_text_only",
        ))
        self.assertTrue(profile.operation_frame(
            "mixed_sequence", "#0 sqlite3_step\n#1 fathomdb_engine::{impl#18}::erase_source",
        ))
        self.assertFalse(profile.operation_frame(
            "mixed_sequence", "#0 futex_wait\n#1 fathomdb_engine::reader_worker_loop",
        ))

    def test_completed_operations_requires_matching_profile_path(self):
        self.assertEqual(profile.completed_operations(
            "text", "PROFILE_READY text\nPROFILE_DONE text operations=12\n",
        ), 12)
        with self.assertRaisesRegex(ValueError, "completion"):
            profile.completed_operations("mixed_sequence", "PROFILE_DONE text operations=12")

    def test_binary_binding_rejects_tampered_receipt_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = b"built Rust test binary"
            (root / "slice135_pilot_workload").write_bytes(binary)
            source_sha = "a" * 40
            protocol = {"source_sha": source_sha,
                        "artifact_sha256": {"slice135_pilot_workload": hashlib.sha256(binary).hexdigest()}}
            (root / "pilot-protocol.json").write_text(json.dumps(protocol))
            self.assertEqual(profile.bound_binary(root, source_sha).read_bytes(), binary)
            (root / "slice135_pilot_workload").write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "binary"):
                profile.bound_binary(root, source_sha)


if __name__ == "__main__":
    unittest.main()
