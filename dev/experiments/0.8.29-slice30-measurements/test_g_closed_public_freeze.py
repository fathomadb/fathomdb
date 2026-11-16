"""Actual public CLI common-worker controls; no frozen model/native inputs are claimed."""

from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_closed_entry
import g_runner

MODE = "closed-copy-calibration"


def argv(root):
    values = [str(Path(g_runner.__file__)), "freeze"]
    for field in ("corpus", "queries", "license", "ordinary", "diagnostic", "build-receipt",
                  "model-cache", "allocation", "out"):
        value = root if field == "allocation" else root / field
        values += ["--" + field, str(value)]
    return values + ["--gpu-uuid", "GPU-protocol-only", "--gpu-ordinal", "0",
                     "--preparation", "closed-copy-v1", "--stage", MODE,
                     "--reuse-completed-from", str(root / "protocol-original")]


def controlled_worker(marker):
    return [sys.executable, "-c", "\n".join([
        "import json,pathlib,sys,time",
        f"time.sleep(.06);pathlib.Path({str(marker)!r}).touch()",
        "value=dict(campaign_uuid='protocol-public-freeze',source_sha='protocol-only',root='protocol-root')",
        "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps(dict(complete=True,value=value)))",
    ])]


class GClosedPublicFreezeTests(unittest.TestCase):
    def test_public_freeze_charges_real_blocking_worker_before_returning(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 1845130877940, "inflight": None, "cell_used_ns": {}, "common_used_ns": {}}))
            marker = root / "worker-ran"
            # Typed entry controls only; actual worker ownership/deadline is exercised.
            with patch.dict(os.environ, {"S30_TASK": str(root)}), \
                    patch("g_boundary.require_known_containment"), patch.object(sys, "argv", argv(root)), \
                    patch.object(g_closed_entry, "worker_command", return_value=controlled_worker(marker)), \
                    redirect_stdout(io.StringIO()):
                g_runner.main()
            self.assertTrue(marker.exists())
            budget = json.loads((root / "g-budget.json").read_text())
            self.assertGreater(budget["used_ns"], 1845130877940,
                               "public freeze returned after uncharged blocking work")
            self.assertEqual(budget["used_ns"] - 1845130877940, budget["common_used_ns"][MODE])
            self.assertGreaterEqual(budget["common_used_ns"][MODE], 60_000_000)

    def test_public_freeze_refuses_before_worker_when_common_allowance_is_consumed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 2000 * 10**9, "inflight": None, "cell_used_ns": {},
                "common_used_ns": {MODE: 600 * 10**9}}))
            marker = root / "forbidden"
            failure = None
            with patch.dict(os.environ, {"S30_TASK": str(root)}), \
                    patch("g_boundary.require_known_containment"), patch.object(sys, "argv", argv(root)), \
                    patch.object(g_closed_entry, "worker_command", return_value=controlled_worker(marker)), \
                    redirect_stdout(io.StringIO()):
                try:
                    g_runner.main()
                except RuntimeError as error:
                    failure = str(error)
            self.assertFalse(marker.exists(), "public freeze renewed the common reserve")
            self.assertIsNotNone(failure)
            self.assertIn("common_time_exhausted", failure)
