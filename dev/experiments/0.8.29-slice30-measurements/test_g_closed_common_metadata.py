"""Frozen/prepared/report metadata shares the common storage reserve outside results."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_closed_common
import g_closed_entry


class GClosedCommonMetadataTests(unittest.TestCase):
    def test_actual_external_report_growth_is_retained_failed_and_blocks_next_worker(self):
        with tempfile.TemporaryDirectory() as temporary:
            allocation = Path(temporary)
            frozen = allocation / "frozen"
            frozen.mkdir()
            (frozen / "manifest.json").write_text(json.dumps({"allocation": str(allocation),
                "preparation": "closed-copy-v1", "stage": "closed-copy-calibration"}))
            calls = allocation / "calls"
            worker = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sys",
                f"p=pathlib.Path({str(calls)!r});p.write_text(p.read_text()+'call\\n' if p.exists() else 'call\\n')",
                f"pathlib.Path({str(frozen/'large-prepared-report.json')!r}).write_bytes(b'x'*131072)",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps(dict(complete=True,value={})))",
            ])]
            failure = None
            with patch.dict(os.environ, {"S30_TASK": str(allocation)}), \
                    patch("g_boundary.require_known_containment"), \
                    patch.object(g_closed_entry, "worker_command", return_value=worker), \
                    patch.object(g_closed_common, "COMMON_BYTES", 65536):
                try:
                    g_closed_entry.report_entry(frozen)
                except RuntimeError as error:
                    failure = str(error)
                self.assertIsNotNone(failure, "common reserve ignored frozen-root metadata growth")
                self.assertIn("common_storage_exhausted", failure)
                self.assertEqual((frozen / "large-prepared-report.json").stat().st_size, 131072)
                with self.assertRaisesRegex(RuntimeError, "common_storage_exhausted"):
                    g_closed_entry.report_entry(frozen)
            self.assertEqual(calls.read_text().splitlines(), ["call"])
            budget = json.loads((allocation / "g-budget.json").read_text())
            self.assertGreater(budget["used_ns"], 0)
            self.assertIsNone(budget["inflight"])
