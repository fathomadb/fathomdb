"""Prospective reviewed totals propagate to public common work and the outer guardian."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_closed_entry
from g_closed_guard import discover
from test_g_closed_public_freeze import controlled_worker


class GClosedProfileEntryTests(unittest.TestCase):
    def test_public_profile_common_worker_and_outer_guard_use_same_retained_totals(self):
        with tempfile.TemporaryDirectory() as temporary:
            allocation = Path(temporary)
            review = allocation / "review.json"
            review.write_text(json.dumps({"schema": "slice30-closed-write-review/v1",
                "admission_scope": "G-only", "review": {"verdict": "ACCEPT", "reviewer": "fixture-review", "subject": "fixture-table"},
                "limits": {"campaign_seconds": 43200, "active_bytes": 128*1024**3}}))
            (allocation / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 15000*10**9, "inflight": None, "cell_used_ns": {}, "common_used_ns": {}}))
            marker = allocation / "actual-common-worker"
            options = dict(allocation=str(allocation), out=str(allocation / "frozen"),
                preparation="closed-copy-v1", stage="closed-copy-write-matrix", cost_review_from=str(review))
            try:
                with patch.dict(os.environ, {"S30_TASK": str(allocation)}), \
                    patch("g_boundary.require_known_containment"), \
                        patch.object(g_closed_entry, "worker_command", return_value=controlled_worker(marker)):
                    g_closed_entry.freeze_entry(options)
            except RuntimeError as error:
                self.fail(f"reviewed prospective public common allowance refused: {error}")
            self.assertTrue(marker.exists())
            budget = json.loads((allocation / "g-budget.json").read_text())
            self.assertGreaterEqual(budget["common_used_ns"][options["stage"]], 60_000_000)
            self.assertGreater(budget["used_ns"], 15000*10**9)
            runner = Path(g_closed_entry.__file__).with_name("g_runner.py")
            command = [sys.executable, str(runner), "freeze"]
            for key, value in options.items():
                command.extend(["--"+key.replace("_", "-"), value])
            profile = discover(command)
            self.assertEqual(profile.get("limits"), {"campaign_seconds": 43200, "active_bytes": 128*1024**3},
                             "outer guardian silently reverted to historical shared limits")
