"""Operation-frame classification for the Slice 115 GDB fallback."""

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "slice115_profile.py"
SPEC = importlib.util.spec_from_file_location("slice115_profile", SCRIPT)
assert SPEC and SPEC.loader
profiler = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(profiler)


class StackClassificationTests(unittest.TestCase):
    def test_operation_frame_is_required(self):
        self.assertTrue(profiler.operation_frame("graph_evidence", "#0 sqlite3_step\n#1 fathomdb_engine::search_api::Engine::resolve_graph_evidence"))
        self.assertFalse(profiler.operation_frame("graph_evidence", "#0 sqlite3_step\n#1 slice115_workload::graph_fixture"))

    def test_attribution_does_not_count_harness_setup(self):
        self.assertFalse(profiler.operation_frame("canonical_write", "#0 sqlite3_open\n#1 slice115_workload::one"))
        self.assertTrue(profiler.operation_frame("canonical_write", "#0 sqlite3_step\n#1 fathomdb_engine::write::Engine::write"))

    def test_control_count_requires_matching_completed_path(self):
        output = 'PROFILE_READY erasure\nPROFILE_DONE erasure operations=42\n'
        self.assertEqual(profiler.completed_operations("erasure", output), 42)
        with self.assertRaisesRegex(ValueError, "completion"):
            profiler.completed_operations("close", output)


if __name__ == "__main__":
    unittest.main()
