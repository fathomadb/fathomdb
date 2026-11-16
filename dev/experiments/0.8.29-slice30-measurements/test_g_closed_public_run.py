"""Public run common-entry wiring with real workers and synthetic readiness observations."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_closed_entry
import g_freeze
import g_runner
from g_reuse import collect_reuse
from test_g_closed_reuse import normal_protocol
from test_g_normal_resume import dump

MODE = "closed-copy-calibration"


class GClosedPublicRunTests(unittest.TestCase):
    def test_public_run_preflight_and_finish_use_common_reserve_not_primary_allowance(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _rows):
                frozen = root / "new-frozen"
                manifest = dict(current, root=str(frozen), source_sha=g_freeze.git("rev-parse", "HEAD"),
                    preparation="closed-copy-v1", stage=MODE, reuse_completed_from=str(normal))
                manifest["reused"] = collect_reuse(manifest, normal)
                dump(frozen / "manifest.json", manifest)
                prior = 1845130877940
                dump(root / "g-budget.json", {"schema": "slice30-gd-budget/v1", "used_ns": prior,
                    "inflight": None, "cell_used_ns": {"g-r-n100000-r1": 42_000_000}, "common_used_ns": {}})
                def controller(_manifest, _root):
                    rows = [manifest["reused"].get(cell["id"], {"id": cell["id"], "cell": cell,
                            "status": "not_run_stage", "complete": False}) for cell in manifest["plan"]]
                    dump(frozen / "results/g-outcomes.json", rows)
                    return rows
                def worker(job):
                    return [sys.executable, "-c", "\n".join([
                        "import json,pathlib,sys,time",
                        f"j=json.loads(pathlib.Path({str(job)!r}).read_text())",
                        f"root=pathlib.Path({str(frozen)!r});time.sleep(.06)",
                        "value=json.loads((root/'manifest.json').read_text()) if j['action']=='preflight' else {'environment_ok':True,'protocol_only':True}",
                        "if j['action']=='finish':(root/'last-run.json').write_text(json.dumps(value))",
                        "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps(dict(complete=True,value=value)))",
                    ])]
                with patch.dict(os.environ, {"S30_TASK": str(root)}), \
                    patch("g_boundary.require_known_containment"), patch.object(g_runner, "run_prepared", controller), \
                        patch.object(g_closed_entry, "worker_command", worker):
                    rows = g_runner.run_frozen(frozen)
                self.assertEqual(len(rows), 84)
                budget = json.loads((root / "g-budget.json").read_text())
                self.assertGreaterEqual(budget.get("common_used_ns", {}).get(MODE, 0), 120_000_000,
                                        "public run preflight/reporting is outside the common reserve")
                self.assertEqual(budget["cell_used_ns"], {"g-r-n100000-r1": 42_000_000},
                                 "common entry renewed/double-charged a primary allowance")
                self.assertEqual(budget["used_ns"]-prior, budget["common_used_ns"][MODE])
                self.assertIsNone(budget["inflight"])
