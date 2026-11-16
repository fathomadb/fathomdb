"""Partial write successors retain original qualified identities and shared closure."""

from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

from g_campaign import run_g_plan
from g_reuse import collect_reuse
from g_runner import qualify_step
from g_serial_storage import capture_closure
from test_g_closed_modes import harmless
from test_g_closed_reuse import normal_protocol
from test_g_closed_r1_reuse import r1_protocol
from test_g_closed_write_gate import setup
from test_g_normal_resume import dump


def write_protocol(root, current, normal):
    r1, _, _ = r1_protocol(root, current, normal)
    current["stage"] = "closed-copy-write-matrix"
    original = root / "write-original"
    manifest = dict(current, root=str(original), campaign_uuid="original-write-uuid",
                    reuse_completed_from=str(r1))
    manifest["reused"] = collect_reuse(current, r1)
    cell = next(row for row in current["plan"] if row["id"] == "g-w-b1-s1-engine-text-r0")
    prior = manifest["reused"]["g-r-n100000-r1"]
    value = deepcopy(json.loads(Path(prior["attempt"], "result.json").read_text()))
    attempt = original / "results" / cell["id"] / "attempt-0001"
    step = value["steps"][0]
    step.update(family="G-W", config=str(attempt / "native-config.json"))
    dump(Path(step["config"]), dict(cell, family="G-W"))
    step["result"]["open_reports"] = step["result"]["open_reports"][:1]
    step["result"].update(sample_count=1000, warmup_count=500, first_call_count=0)
    assert qualify_step(step, manifest) == [], "invalid typed write scaffold, not a semantic RED"
    dump(attempt / "result.json", value)
    row = {"id": cell["id"], "cell": cell, "status": "complete", "attempt": str(attempt),
           "exit": 0, "child_exit": 0, "pid": step["pid"], "cleanup_uncertain": False,
           "live_owned_pids": [], "elapsed_ns": 123456789}
    dump(original / "manifest.json", manifest)
    dump(original / "results/g-outcomes.json", [*manifest["reused"].values(), row])
    receipt = deepcopy(json.loads((r1 / "guardian.json").read_text()))
    dump(original / "guardian.json", receipt)
    dump(original / "last-run.json", {"environment_ok": True, "stage_complete": False,
         "campaign_complete": False, "guardian_receipt": str(original / "guardian.json")})
    budget = Path(current["allocation"], "g-budget.json")
    dump(budget, {"schema": "slice30-gd-budget/v1", "used_ns": 987654321, "inflight": None,
         "cell_used_ns": {cell["id"]: 123456789},
         "serial_closed_cases": {cell["id"]: capture_closure(row)}})
    return original, manifest, row


class GClosedWriteReuseTests(unittest.TestCase):
    def test_successor_retains_thirteen_originals_and_dispatches_only_next_write(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _):
                original, manifest, row = write_protocol(root, current, normal)
                references = collect_reuse(current, original)
                self.assertEqual(len(references), 13)
                self.assertEqual({key: references[key] for key in manifest["reused"]}, manifest["reused"])
                retained = references[row["id"]]
                self.assertEqual(retained["attempt"], row["attempt"])
                self.assertEqual(retained["origin"]["source_sha"], manifest["source_sha"])
                self.assertEqual(retained["origin"]["campaign_uuid"], "original-write-uuid")
                spec, _ = setup()
                spec["_reviewed_costs"]["r1"] = references["g-r-n100000-r1"]["origin"]
                spec["_max_new_writes"] = 1
                rows = run_g_plan(spec, root / "successor/results", harmless(root / "calls"),
                                  allocation_root=Path(current["allocation"]), reused=references)
                self.assertEqual((root / "calls").read_text().splitlines(), ["g-w-b1-s1-engine-hybrid-r0"])
                self.assertEqual(len(rows), 84)
                self.assertEqual(next(value for value in rows if value["id"] == row["id"]), retained)
                self.assertGreaterEqual(json.loads(Path(current["allocation"], "g-budget.json").read_text())["used_ns"], 987654321)

    def test_unqualified_completed_write_is_not_promoted(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _):
                original, _, row = write_protocol(root, current, normal)
                result = Path(row["attempt"], "result.json")
                value = json.loads(result.read_text())
                value["qualification_issues"] = ["actual_call_population_mismatch"]
                dump(result, value)
                with self.assertRaisesRegex(ValueError, "qualification is incomplete"):
                    collect_reuse(current, original)
