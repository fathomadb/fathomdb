"""Actual controlled guardian errors retain their original diagnostics and receipt."""

import json
from pathlib import Path
import tempfile
import unittest

from g_boundary_entry_tests import unknown_boundary


class GPhaseErrorReportingTests(unittest.TestCase):
    def test_population_error_after_actual_payload_retains_failed_guardian_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            try:
                with unknown_boundary(root):
                    pass
            except FileNotFoundError:
                pass
            path = root / "receipts/unknown-control.json"
            self.assertTrue(path.exists(), "handled population error crashed reporting instead of retaining guardian receipt")
            receipt = json.loads(path.read_text())
            self.assertTrue(receipt["cleanup_uncertain"])
            self.assertEqual(receipt["child_exit"], 0)
            self.assertIn("controlled unknown population after payload completed", receipt["cleanup_error"])
            self.assertIn("controlled unknown population after payload completed",
                          (root / "logs/unknown-control.log").read_text())
