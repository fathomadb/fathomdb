"""Closed-profile freeze routing with declared synthetic entry metadata only."""

import json
from pathlib import Path
import tempfile
import unittest

from g_freeze import freeze
from test_g_closed_plan import ORIGINAL
from test_g_closed_reuse import normal_protocol
from test_g_closed_copy import tiny_source


class GClosedFreezeTests(unittest.TestCase):
    def test_known_normal_root_freezes_calibration_without_retagging_original11(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _rows):
                source = tiny_source(root)
                result_path = root / "seed/results/g1-n100000/attempt-0001/result.json"
                value = json.loads(result_path.read_text())
                value["steps"][0]["closed_store"] = source["files"][""]["path"]
                value["steps"][0]["span_coverage"] = source["coverage"]
                result_path.write_text(json.dumps(value))
                options = {key: value["path"] for key, value in current["inputs"].items()}
                options.update({key: value["path"] for key, value in current["binaries"].items()})
                cache = Path(current["model_files"]["config.json"]["path"]).parent
                options.update(build_receipt=current["build_receipt"]["path"], model_cache=str(cache),
                    allocation=str(root), out=str(root / "closed-calibration"), gpu_uuid=current["gpu"]["uuid"],
                    gpu_ordinal=0, reuse_completed_from=str(normal), preparation="closed-copy-v1",
                    stage="closed-copy-calibration")
                try:
                    manifest = freeze(options)
                except ValueError as error:
                    self.fail(f"known original11 closed-calibration entry is refused: {error}")
                self.assertEqual(set(manifest["reused"]), ORIGINAL)
                self.assertEqual(manifest["preparation"], "closed-copy-v1")
                self.assertEqual(manifest["stage"], "closed-copy-calibration")
                self.assertEqual(manifest["closed_subject"]["files"][""], source["files"][""])
                self.assertEqual(manifest["closed_subject"]["coverage"], source["coverage"])
                self.assertEqual(manifest["closed_limits"]["campaign_seconds"], 14400)
                self.assertEqual(manifest["closed_limits"]["active_bytes"], 16 * 1024**3)
                self.assertTrue(all(manifest["reused"][key]["cell"]["cell_seconds"] == 900 for key in ORIGINAL))
