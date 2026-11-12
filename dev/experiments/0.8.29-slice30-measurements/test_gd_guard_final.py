"""Final observations must govern the disposition of real small owned children."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from gd_resources import run_guarded


class FinalObservationTests(unittest.TestCase):
    def test_immediate_successful_child_over_cap_is_resource_stopped(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payload = root / "payload"
            command = [sys.executable, "-c",
                       f"from pathlib import Path; Path({str(payload)!r}).write_bytes(b'x'*131072); print('complete',flush=True)"]
            result = run_guarded(command, root, roots=[str(root)], baseline=0,
                                 max_owned=65536, interval=30)
            print(json.dumps(result), flush=True)
            self.assertEqual(result["child_exit"], 0)
            self.assertEqual(payload.stat().st_size, 131072)
            self.assertIn("complete", (root / "logs/phase.log").read_text())
            self.assertEqual(result["exit"], 124)
            self.assertEqual(result["resource_stop"], "owned_capacity")
            saved = json.loads((root / "receipts/phase.json").read_text())
            self.assertEqual(saved["exit"], 124)

    def test_unavailable_final_observation_cannot_leave_success(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            measured = root / "empty-fixture"
            measured.mkdir()
            command = [sys.executable, "-c",
                       f"from pathlib import Path; Path({str(measured)!r}).rmdir(); print('fixture-complete',flush=True)"]
            result = run_guarded(command, root, roots=[str(measured)], baseline=0, interval=30)
            print(json.dumps(result), flush=True)
            self.assertEqual(result["child_exit"], 0)
            self.assertIn("fixture-complete", (root / "logs/phase.log").read_text())
            self.assertIn("final_sample_error", result)
            self.assertEqual(result["exit"], 124)
            self.assertEqual(result["resource_stop"], "final_observation_unavailable")


if __name__ == "__main__":
    unittest.main()
