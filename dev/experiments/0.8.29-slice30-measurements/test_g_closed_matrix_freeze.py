"""Freeze binds prospective review metadata and12 original references; run rechecks actual costs."""

import json
from pathlib import Path
import tempfile
import unittest

from g_freeze import freeze, identify
from test_g_closed_copy import tiny_source
from test_g_closed_r1_reuse import r1_protocol
from test_g_closed_reuse import normal_protocol
from test_g_normal_resume import dump


class GClosedMatrixFreezeTests(unittest.TestCase):
    def test_freeze_binds_reviewed_prospective_plan_without_retagging_original12(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _rows):
                source = tiny_source(root)
                result_path = root / "seed/results/g1-n100000/attempt-0001/result.json"
                value = json.loads(result_path.read_text())
                value["steps"][0].update(closed_store=source["files"][""]["path"], span_coverage=source["coverage"])
                dump(result_path, value)
                r1, original, row = r1_protocol(root, current, normal)
                review = {"schema": "slice30-closed-write-review/v1", "admission_scope": "G-only",
                    "review": {"verdict": "ACCEPT", "reviewer": "fixture-review", "subject": "fixture-table"},
                    "limits": {"campaign_seconds": 43200, "active_bytes": 128*1024**3}, "observation_wait_ms": 50}
                review_path = root / "review.json"
                dump(review_path, review)
                options = {key: record["path"] for key, record in current["inputs"].items()}
                options.update({key: record["path"] for key, record in current["binaries"].items()})
                options.update(build_receipt=current["build_receipt"]["path"],
                    model_cache=str(Path(current["model_files"]["config.json"]["path"]).parent),
                    allocation=str(root), out=str(root / "matrix"), gpu_uuid=current["gpu"]["uuid"], gpu_ordinal=0,
                    preparation="closed-copy-v1", stage="closed-copy-write-matrix", reuse_completed_from=str(r1),
                    calibration_from=original["calibration_from"], cost_review_from=str(review_path))
                try:
                    manifest = freeze(options)
                except ValueError as error:
                    self.fail(f"named prospective freeze refused its exact original12 transition: {error}")
                self.assertEqual(len(manifest["reused"]), 12)
                self.assertEqual(manifest["reused"][row["id"]]["cell"], row["cell"])
                self.assertEqual(manifest["reused"][row["id"]]["attempt"], row["attempt"])
                self.assertEqual(manifest["cost_review"], review)
                self.assertEqual(manifest["cost_review_pin"], identify(review_path))
                self.assertEqual(manifest["closed_limits"]["campaign_seconds"], 43200)
                self.assertEqual(manifest["closed_limits"]["active_bytes"], 128*1024**3)
                self.assertTrue(all(cell["observation_wait_ms"] == 50 for cell in manifest["plan"] if cell["stage"] == "G-W"))
                self.assertTrue(all(manifest["reused"][key]["cell"]["cell_seconds"] == 900
                                    for key in manifest["reused"] if key != row["id"]))
