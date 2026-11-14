"""Public supervised preparation binds qualified calibration/R1 and the reviewed whole table."""

from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import g_closed_entry
import g_closed_costs
from g_campaign import active_bytes, make_g_plan
from g_closed_forecast import forecast_closed_write
from g_reuse import collect_reuse
from test_g_closed_cost_loading import calibration_protocol
from test_g_closed_r1_reuse import r1_protocol
from test_g_closed_reuse import normal_protocol
from test_g_normal_resume import dump


def read_records(references):
    for repeat in (0, 1):
        row = references[f"g-r-n100000-r{repeat}"]
        result_path = Path(row["attempt"], "result.json")
        value = json.loads(result_path.read_text())
        result = value["steps"][0]["result"]
        result["passes"] = [{"adapter": adapter, "route": route, "requests": 1000,
            "wall_ns": (10 if route == "text" else 20 if route == "hybrid" else 30)*10**9}
            for adapter in ("engine", "sdk") for route in ("text", "hybrid", "embed")]
        result["samples"] = [{"adapter": adapter, "route": "text", "ok": True,
                              "elapsed_ns": 10_000_000} for adapter in ("engine", "sdk")]
        dump(result_path, value)


class GClosedMatrixEntryTests(unittest.TestCase):
    def test_current_supervised_entry_recomputes_binds_and_funds_the_recorded_full_table(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _rows):
                calibration = calibration_protocol(root, current, normal)
                r1, original, _ = r1_protocol(root, current, normal)
                current["calibration_from"] = original["calibration_from"] = str(calibration)
                dump(r1 / "manifest.json", original)
                current.update(root=str(root / "matrix"), stage="closed-copy-write-matrix",
                    reuse_completed_from=str(r1), calibration_from=str(calibration),
                    closed_limits={"campaign_seconds": 43200, "active_bytes": 128*1024**3})
                current["reused"] = collect_reuse(current, r1)
                read_records(current["reused"])
                # Fixture cost rows are typed protocol data; hand-cost oracles are separate tests.
                with patch.object(g_closed_costs, "active_bytes", return_value=715233040):
                    qualified = g_closed_costs.load_calibration(calibration, current)
                spec = dict(current["spec"], closed_mode=current["stage"])
                review = {"schema": "slice30-closed-write-review/v1", "admission_scope": "G-only",
                    "review": {"verdict": "ACCEPT", "reviewer": "fixture-independent-review", "subject": "fixture-cost-table"},
                    "calibration": qualified["origin"], "r1": current["reused"]["g-r-n100000-r1"]["origin"],
                    "limits": current["closed_limits"], "observation_wait_ms": 50,
                    "combined_feasibility": "UNKNOWN",
                    "other_phases": [{"phase": phase, "status": "HELD_COST_UNAVAILABLE", "forecast_ns": None,
                                      "new_bytes": None} for phase in ("D", "M")], "g_table": {}}
                spec.update(_closed_limits=review["limits"], _reviewed_costs=review,
                            _closed_observation_wait_ms=50)
                current.update(spec=spec, plan=make_g_plan(spec))
                prior = 1845130877940
                dump(root / "g-budget.json", {"schema": "slice30-gd-budget/v1", "used_ns": prior,
                    "inflight": {"cell": "g-common-closed-copy-write-matrix"}, "cell_used_ns": {},
                    "common_used_ns": {current["stage"]: 123}, "exploratory_seed100k": {"consumed": True}})
                retained = active_bytes(root)
                review["allocation_snapshot"] = {"used_ns": prior, "retained_bytes": retained,
                    "common_used_ns": 123, "remaining_common_ns": 600*10**9-123,
                    "remaining_shared_ns": 43200*10**9-prior,
                    "remaining_retained_bytes": 128*1024**3-retained}
                for cell in current["plan"]:
                    if cell["stage"] != "G-W":
                        continue
                    adapter, route, repeat = cell["adapters"][0], cell["routes"][0], cell["repeat"]
                    read = current["reused"][f"g-r-n100000-r{repeat}"]
                    count = 9006
                    observed = {"calibration": qualified["costs"][adapter],
                        "read_repeat": repeat, "read_adapter": adapter, "read_route": route,
                        "route_requests": 1000, "route_wall_ns": (10 if route=="text" else 20 if route=="hybrid" else 30)*10**9,
                        "raw_bytes_per_record": (active_bytes(read["attempt"])+count-1)//count}
                    legacy = {"components": {"seed_ns": 2*current["reused"]["g1-n100000"]["elapsed_ns"]},
                              "allowed": False, "reason": "original_reseed_forecast"}
                    prediction = forecast_closed_write(cell, observed, 43200*10**9-prior, retained, legacy)
                    review["g_table"][cell["id"]] = {"cell": cell, "prediction": prediction}
                review_path = root / "review.json"
                dump(review_path, review)
                Path(current["root"]).mkdir()
                current["cost_review_from"] = str(review_path)
                current["cost_review"] = deepcopy(review)
                current["inputs"] = dict(current["inputs"])
                current["binaries"] = dict(current["binaries"])
                with patch("g_freeze.validate_frozen", return_value=current), \
                        patch("g_gpu.inspect_gpu", return_value={"applications": []}), \
                        patch.object(g_closed_costs, "active_bytes", return_value=715233040):
                    try:
                        prepared = g_closed_entry._preflight(Path(current["root"]))
                    except ValueError as error:
                        self.fail(f"valid reviewed qualified matrix refused at current entry: {error}")
                self.assertEqual(len(prepared.get("_closed_predictions", {})), 72)
                self.assertEqual(prepared.get("_reviewed_costs"), review)
                self.assertEqual(prepared.get("_calibration_origin"), qualified["origin"])
                self.assertEqual(prepared.get("_closed_limits"), review["limits"])
                self.assertEqual(prepared["reused"], current["reused"])
                self.assertEqual(prepared["_reviewed_costs"]["combined_feasibility"], "UNKNOWN")
                self.assertTrue(Path(prepared["_prepared_manifest"]).exists())
