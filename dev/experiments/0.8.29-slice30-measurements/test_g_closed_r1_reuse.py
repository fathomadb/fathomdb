"""Qualified R1 joins the original11 by exact reference; calibration remains separate."""

from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

import g_freeze
from g_reuse import collect_reuse
from g_runner import qualify_step
from test_g_closed_plan import ORIGINAL
from test_g_closed_reuse import normal_protocol
from test_g_normal_resume import dump


def r1_protocol(root, current, normal):
    """Declare a typed already-qualified R1 record, not actual Default/GPU execution."""
    r1_root = root / "r1-original"
    current["calibration_from"] = str(root / "calibration-protocol")
    manifest = dict(current, root=str(r1_root), source_sha=g_freeze.git("rev-parse", "HEAD"),
                    campaign_uuid="r1-protocol-uuid", stage="closed-copy-read-r1",
                    reuse_completed_from=str(normal))
    manifest["reused"] = collect_reuse(current, normal)
    dump(r1_root / "manifest.json", manifest)
    cell = next(row for row in current["plan"] if row["id"] == "g-r-n100000-r1")
    prior = manifest["reused"]["g-r-n100000-r0"]
    value = deepcopy(json.loads(Path(prior["attempt"], "result.json").read_text()))
    attempt = r1_root / "results" / cell["id"] / "attempt-0001"
    step = value["steps"][0]
    step["config"] = str(attempt / "native-config.json")
    dump(Path(step["config"]), dict(cell, family="G-R"))
    assert qualify_step(step, manifest) == [], "invalid typed R1 scaffold, not a semantic RED"
    dump(attempt / "result.json", value)
    row = {"id": cell["id"], "cell": cell, "status": "complete", "attempt": str(attempt), "exit": 0,
           "cleanup_uncertain": False, "live_owned_pids": [], "elapsed_ns": 709385925507}
    dump(r1_root / "results/g-outcomes.json", [*manifest["reused"].values(), row])
    receipt = r1_root / "guardian.json"
    dump(receipt, {"exit": 1, "sha_start": manifest["source_sha"], "sha_end": manifest["source_sha"],
        "delta_start": "", "delta_end": "", "resource_stop": None, "cleanup_uncertain": False,
        "live_owned_pids": [], "boundary": {"status": "quiescent", "populated_after": False,
                                           "removed_exact_empty_boundary": True}})
    dump(r1_root / "last-run.json", {"environment_ok": True, "stage_complete": True,
                                   "guardian_receipt": str(receipt)})
    return r1_root, manifest, row


class GClosedR1ReferenceTests(unittest.TestCase):
    def test_write_successor_preserves_original11_plus_qualified_r1_without_copy_or_retag(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _rows):
                r1_root, manifest, row = r1_protocol(root, current, normal)
                successor = dict(current, stage="closed-copy-write-matrix")
                try:
                    references = collect_reuse(successor, r1_root)
                except ValueError as error:
                    self.fail(f"valid qualified R1 transition refused: {error}")
                self.assertEqual(set(references), ORIGINAL | {"g-r-n100000-r1"})
                r1 = references["g-r-n100000-r1"]
                self.assertEqual(r1["cell"], row["cell"])
                self.assertEqual(r1["attempt"], row["attempt"])
                self.assertEqual(r1["cell"]["cell_seconds"], 1500)
                self.assertEqual(r1["origin"]["source_sha"], manifest["source_sha"])
                self.assertEqual(r1["origin"]["campaign_uuid"], "r1-protocol-uuid")
                self.assertTrue(all(references[key]["cell"]["cell_seconds"] == 900 for key in ORIGINAL))
                self.assertFalse(any(key.startswith("g-cal-") for key in references))
