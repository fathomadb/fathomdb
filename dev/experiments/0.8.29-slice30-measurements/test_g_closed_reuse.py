"""Original11 reference protocol; typed fixture observations are not GPU qualification."""

from contextlib import contextmanager
import copy
import json
from pathlib import Path
import tempfile
import unittest

from g_campaign import make_g_plan
from g_reuse import collect_reuse
from g_runner import qualify_step
from test_g_closed_plan import ORIGINAL, SPEC
from test_g_normal_resume import accepted_protocol_chain, dump

NORMAL_SOURCE = "29de06806cc22ab6b8e67f970133f856f3c42f78"
NORMAL_UUID = "4fb7a91f-2d41-445d-a132-258334a8f3fe"


@contextmanager
def normal_protocol(root):
    """Extend the existing synthetic input/device fixture, executing actual reuse/qualifier code."""
    with accepted_protocol_chain(root) as (options, six):
        seed_manifest = json.loads((root / "seed/manifest.json").read_text())
        normal = root / "normal-original"
        manifest = dict(seed_manifest, root=str(normal), source_sha=NORMAL_SOURCE,
                        campaign_uuid=NORMAL_UUID, reuse_completed_from=str(root / "seed"))
        manifest.pop("stage")
        dump(normal / "manifest.json", manifest)
        dump(normal / "last-run.json", {"environment_ok": True, "campaign_complete": False})
        template = json.loads(Path(six["g-r-n10000-r0"]["attempt"], "result.json").read_text())
        rows = list(six.values())
        for cell in manifest["plan"]:
            if cell["id"] not in ORIGINAL or cell["id"] in six:
                continue
            attempt = normal / "results" / cell["id"] / "attempt-0001"
            value = copy.deepcopy(template)
            step = value["steps"][0]
            family = cell["stage"]
            config = dict(cell, family=family)
            step.update(family=family, config=str(attempt / "native-config.json"), output=str(attempt / "read"))
            dump(Path(step["config"]), config)
            binary = manifest["binaries"][cell["artifact"]]["path"]
            step["executable"] = binary
            for report in step["result"]["open_reports"]:
                report["executable"] = binary
            for fixture in step["result"]["fixtures"]:
                fixture.update(natural_records=100000, vector_rows=100000,
                               physical_vector_rows=100000, span_rows=100000)
            scale = len(cell["adapters"]) * len(cell["routes"])
            step["result"].update(sample_count=cell["samples"] * scale,
                warmup_count=cell["warmup"] * scale, first_call_count=int(cell["record_first_call"]) * scale)
            if cell["observe"]:
                step["result"]["samples"] = []
                step["result"]["observation_batches"] = []
            assert qualify_step(step, manifest) == [], "invalid protocol scaffold, not a semantic RED"
            dump(attempt / "result.json", value)
            rows.append({"id": cell["id"], "cell": cell, "status": "complete", "attempt": str(attempt),
                         "exit": 0, "cleanup_uncertain": False, "live_owned_pids": [], "elapsed_ns": 1})
        dump(normal / "results/g-outcomes.json", rows)
        current = dict(manifest, preparation="closed-copy-v1", stage="closed-copy-calibration",
                       spec=dict(SPEC, preparation="closed-copy-v1"),
                       plan=make_g_plan(dict(SPEC, preparation="closed-copy-v1")))
        yield current, normal, rows


class GClosedReuseTests(unittest.TestCase):
    def test_accepted11_original_records_survive_prospective_unmeasured_plan_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            with normal_protocol(Path(temporary)) as (current, normal, rows):
                try:
                    actual = collect_reuse(current, normal)
                except ValueError as error:
                    self.fail(f"valid accepted11 successor refused instead of exact per-record reuse: {error}")
                self.assertEqual(set(actual), ORIGINAL)
                originals = {row["id"]: row for row in rows}
                for identifier, row in actual.items():
                    self.assertEqual(row["cell"], originals[identifier]["cell"])
                    self.assertEqual(row["attempt"], originals[identifier]["attempt"])
                    self.assertEqual(row["cell"]["cell_seconds"], 900)
                    if identifier not in {"g0-smoke", "g1-n10000", "g1-n100000", "g-o-branches",
                                          "g-r-n10000-r0", "g-r-n10000-r1"}:
                        self.assertEqual(row["origin"]["campaign_uuid"], NORMAL_UUID)
                self.assertNotIn("g-r-n100000-r1", actual)
