"""Specific copy/validation stalls remain supervised; inherited preservation, not new RED."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_campaign
from test_g_closed_copy import tiny_source
from test_g_closed_modes import references
from test_g_closed_plan import SPEC


class GClosedStallTests(unittest.TestCase):
    def test_copy_and_validation_stalls_never_dispatch_native_and_charge_cleanup(self):
        for stage in ("copy", "validation"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                source = tiny_source(root)
                original = Path(source["files"][""]["path"]).read_bytes()
                ready, forbidden = root / "stalled", root / "native-forbidden"
                prior = 1500 * 10**9 - 800_000_000
                (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                    "used_ns": 2000 * 10**9, "inflight": None,
                    "cell_used_ns": {"g-r-n100000-r1": prior}}))
                native = [sys.executable, "-c", f"from pathlib import Path;Path({str(forbidden)!r}).touch()"]
                worker = "\n".join([
                    "import json,pathlib,sys,time",
                    f"sys.path.insert(0,{str(Path(g_campaign.__file__).parent)!r})",
                    "import g_closed_copy",
                    "from g_driver import execute_stage",
                    f"ready=pathlib.Path({str(ready)!r})",
                    "def stall():ready.touch();time.sleep(10)",
                    f"source={source!r}",
                    f"stage={stage!r}",
                    "if stage=='copy':",
                    " original_open=pathlib.Path.open",
                    " class Reader:",
                    "  def __enter__(self):return self",
                    "  def __exit__(self,*args):self.reader.close()",
                    "  def read(self,*args):stall();return self.reader.read(*args)",
                    " def opening(path,*args,**kwargs):",
                    "  stream=original_open(path,*args,**kwargs)",
                    "  if str(path)==source['files']['']['path'] and args==('rb',):",
                    "   reader=Reader();reader.reader=stream;return reader",
                    "  return stream",
                    " pathlib.Path.open=opening",
                    "else:",
                    " original_coverage=g_closed_copy.stored_coverage",
                    " def coverage(path):",
                    "  value=original_coverage(path);stall();return value",
                    " g_closed_copy.stored_coverage=coverage",
                    "cell=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                    f"manifest=dict(command={native!r},closed_subject=source,corpus='unused',queries='unused')",
                    "execute_stage(cell,manifest,pathlib.Path(sys.argv[2]),int(sys.argv[3]))",
                ])
                def command(_cell, deadline):
                    # run_g_attempt appends config/output; put the fixed deadline after them in the child.
                    code = worker.replace("int(sys.argv[3])", str(deadline))
                    return [sys.executable, "-c", code]
                spec = dict(SPEC, preparation="closed-copy-v1", closed_mode="closed-copy-read-r1")
                with patch.object(g_campaign, "CLOSED_CLEANUP_RESERVE_NS", 300_000_000):
                    rows = g_campaign.run_g_plan(spec, root / "results", command,
                        allocation_root=root, reused=references())
                row = next(value for value in rows if value["id"] == "g-r-n100000-r1")
                budget = json.loads((root / "g-budget.json").read_text())
                self.assertTrue(ready.exists(), f"{stage} stall was not reached")
                self.assertFalse(forbidden.exists(), f"native dispatch followed stalled {stage}")
                self.assertEqual(row["status"], "failed")
                self.assertIn("timeout", row["error"])
                self.assertFalse(row["cleanup_uncertain"])
                self.assertEqual(row["live_owned_pids"], [])
                self.assertIsNone(budget["inflight"])
                charged = budget["cell_used_ns"][row["id"]] - prior
                self.assertGreater(charged, 0)
                self.assertLessEqual(charged, 800_000_000)
                self.assertEqual(Path(source["files"][""]["path"]).read_bytes(), original)
