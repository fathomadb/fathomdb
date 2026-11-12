"""A real failed du's numeric partial total is not verified capacity evidence."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from gd_resources import measure_root


class DuEvidenceTests(unittest.TestCase):
    def test_unreadable_subtree_partial_total_is_retried_then_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            unreadable = root / "owned-unreadable-fixture"
            unreadable.mkdir()
            (unreadable / "payload").write_bytes(b'x' * 131072)
            unreadable.chmod(0)
            try:
                actual = subprocess.run(["du", "-sB1", str(root)], capture_output=True, text=True)
                print(json.dumps({"command":actual.args,"exit":actual.returncode,
                                  "stdout":actual.stdout,"stderr":actual.stderr}),flush=True)
                self.assertNotEqual(actual.returncode, 0, "fixture must exercise a real du failure")
                self.assertTrue(actual.stdout.split()[0].isdigit())
                self.assertIn("Permission denied", actual.stderr)
                with self.assertRaisesRegex(RuntimeError, "owned size unverifiable") as caught:
                    measure_root(root)
                self.assertIn("attempt 1", str(caught.exception))
                self.assertIn("attempt 2", str(caught.exception))
                self.assertIn("Permission denied", str(caught.exception))
                print(str(caught.exception),flush=True)
            finally:
                unreadable.chmod(0o700)


if __name__ == "__main__":
    unittest.main()
