"""Typed qualification protocol and literal cost loading, never actual GPU evidence."""

from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import g_closed_costs
import g_freeze
from g_reuse import collect_reuse
from g_runner import qualify_step
from test_g_closed_costs import cost_record
from test_g_closed_reuse import normal_protocol
from test_g_normal_resume import dump


def calibration_protocol(root, current, normal):
    """Declare cost/qualification records; current qualifier still runs on each typed record."""
    cal = root / "qualified-calibration"
    manifest = dict(current, root=str(cal), source_sha=g_freeze.git("rev-parse", "HEAD"),
                    campaign_uuid="calibration-protocol-uuid", stage="closed-copy-calibration",
                    reuse_completed_from=str(normal))
    manifest["reused"] = collect_reuse(current, normal)
    current["reused"] = manifest["reused"]
    dump(cal / "manifest.json", manifest)
    receipt = cal / "guardian.json"
    dump(receipt, {"exit": 1, "sha_start": manifest["source_sha"], "sha_end": manifest["source_sha"],
                  "delta_start": "", "delta_end": "", "resource_stop": None, "cleanup_uncertain": False, "live_owned_pids": [],
                  "boundary": {"status": "quiescent", "populated_after": False,
                               "removed_exact_empty_boundary": True}})
    dump(cal / "last-run.json", {"environment_ok": True, "stage_complete": True,
                                 "guardian_receipt": str(receipt)})
    template = json.loads(Path(manifest["reused"]["g-r-n10000-r0"]["attempt"], "result.json").read_text())
    base = next(cell for cell in current["plan"] if cell["id"] == "g-w-b1-s1-engine-text-r0")
    rows = []
    for batch, ratio in ((10, 1), (1, 100)):
        for adapter in ("engine", "sdk"):
            literal = cost_record(adapter, batch, ratio, 40_000_000 if batch == 10 else 5_000_000,
                                  2_000_000, 3_000_000, 4_000_000)
            cell = dict(base, **literal["cell"], calibration=True)
            cell["name"] = cell["id"]
            attempt = cal / "results/calibration" / cell["id"] / "attempt-0001"
            value = literal["actual"]
            step = deepcopy(template["steps"][0])
            native = dict(step["result"], **value["steps"][0]["result"])
            native["warmups"][0].update(ok=True, route="text", adapter=adapter)
            native["open_reports"] = native["open_reports"][:1]
            for fixture in native["fixtures"]:
                fixture.update(natural_records=100000, vector_rows=100000,
                               physical_vector_rows=100000, span_rows=100000)
            native.update(sample_count=1000, warmup_count=500, first_call_count=0)
            step.update(value["steps"][0])
            step.update(result=native, family="G-W", config=str(attempt / "native-config.json"))
            dump(Path(step["config"]), dict(cell, family="G-W"))
            assert qualify_step(step, manifest) == [], "invalid typed protocol scaffold, not semantic RED"
            value.update(complete=True, qualification_issues=[], steps=[step])
            dump(attempt / "result.json", value)
            rows.append({"id": cell["id"], "cell": cell, "status": "complete", "attempt": str(attempt),
                         "exit": 0, "cleanup_uncertain": False, "live_owned_pids": [],
                         "peak_store_bytes": literal["peak_store_bytes"]})
    dump(cal / "results/calibration-outcomes.json", rows)
    return cal


class GClosedCalibrationLoadingTests(unittest.TestCase):
    def test_qualified_four_record_protocol_loads_costs_for_both_adapters(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _rows):
                cal = calibration_protocol(root, current, normal)
                # Synthetic artifact-byte observation only; no large store is fabricated.
                with patch.object(g_closed_costs, "active_bytes", return_value=715233040):
                    actual = g_closed_costs.load_calibration(cal, current)
                self.assertEqual(actual.get("costs", {}).get("engine", {}).get("copy_ns"), 2*10**9,
                                 "actual qualified calibration record loader supplies no matched costs")
                self.assertEqual(len(actual["records"]), 4)
                self.assertEqual(set(actual["costs"]), {"engine", "sdk"})
                self.assertEqual(actual["costs"]["sdk"]["write_row_max_ns"], 5_000_000)
                self.assertEqual(actual["costs"]["engine"]["physical_growth_per_row"], 4096)
                self.assertEqual(actual["origin"]["campaign_uuid"], "calibration-protocol-uuid")
