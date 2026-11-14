"""Common retained growth is charged under a scaled real-file resource control."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_closed_common
from test_g_closed_common import MODE, worker


class GClosedCommonStorageTests(unittest.TestCase):
    def test_retained_growth_fails_common_work_and_refuses_later_operation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payload = root / "common" / "retained-partial"
            command = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sys",
                f"p=pathlib.Path({str(payload)!r});p.parent.mkdir(exist_ok=True);p.write_bytes(b'x'*131072)",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])]
            # Scaled test allowance; production common reserve remains1GiB.
            with patch.object(g_closed_common, "COMMON_BYTES", 65536):
                first = g_closed_common.run_common(root, MODE, command)
                self.assertEqual(first["status"], "failed", "common retained growth escaped its reserve")
                self.assertIn("common_storage_exhausted", first["error"])
                self.assertEqual(payload.stat().st_size, 131072)
                second = g_closed_common.run_common(root, MODE, worker(root / "forbidden"))
                self.assertEqual(second["status"], "not_run_cap")
                self.assertEqual(second["reason"], "common_storage_exhausted")
                self.assertFalse((root / "forbidden").exists())
                budget = json.loads((root / "g-budget.json").read_text())
                self.assertGreater(budget["common_used_ns"][MODE], 0)
