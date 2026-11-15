"""The public matrix entry binds the measured model and uses the serial stage peak."""

from copy import deepcopy
import os
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
from test_g_closed_matrix_entry import read_records
from test_g_closed_r1_reuse import r1_protocol
from test_g_closed_reuse import normal_protocol
from test_g_normal_resume import dump
from test_g_serial_storage import model


class GSerialMatrixEntryTests(unittest.TestCase):
    def test_public_preflight_binds_all72_serial_predictions_without_flat_live_overreservation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with normal_protocol(root) as (current, normal, _):
                calibration = calibration_protocol(root, current, normal)
                r1, original, _ = r1_protocol(root, current, normal)
                current["calibration_from"] = original["calibration_from"] = str(calibration)
                dump(r1/"manifest.json", original)
                current.update(root=str(root/"matrix"), stage="closed-copy-write-matrix",
                    reuse_completed_from=str(r1), calibration_from=str(calibration),
                    closed_limits={"campaign_seconds": 43200, "active_bytes": 128*1024**3})
                current["reused"] = collect_reuse(current, r1)
                read_records(current["reused"])
                with patch.object(g_closed_costs, "active_bytes", return_value=715233040):
                    qualified = g_closed_costs.load_calibration(calibration, current)
                storage = model()
                storage["basis"].update(r0=current["reused"]["g-r-n100000-r0"]["origin"],
                    r1=current["reused"]["g-r-n100000-r1"]["origin"], calibration=qualified["origin"])
                review = {"schema": "slice30-closed-write-review/v1", "admission_scope": "G-only",
                    "review": {"verdict": "ACCEPT", "reviewer": "protocol", "subject": "serial-protocol"},
                    "calibration": qualified["origin"], "r1": storage["basis"]["r1"], "storage_model": storage,
                    "limits": current["closed_limits"], "observation_wait_ms": 50, "combined_feasibility": "UNKNOWN",
                    "other_phases": [{"phase": phase, "status": "HELD_COST_UNAVAILABLE", "forecast_ns": None,
                                      "new_bytes": None} for phase in ("D", "M")], "g_table": {}}
                spec = dict(current["spec"], closed_mode=current["stage"], _closed_limits=review["limits"],
                            _reviewed_costs=review, _closed_observation_wait_ms=50)
                current.update(spec=spec, plan=make_g_plan(spec))
                dump(root/"g-budget.json", {"schema": "slice30-gd-budget/v1", "used_ns": 1,
                    "inflight": {"cell": "g-common-closed-copy-write-matrix"}, "cell_used_ns": {}})
                retained = active_bytes(root)
                review["allocation_snapshot"] = {"used_ns": 1, "retained_bytes": retained, "common_used_ns": 0,
                    "remaining_common_ns": 600*10**9, "remaining_shared_ns": 43200*10**9-1,
                    "remaining_retained_bytes": 128*1024**3-retained}
                for cell in current["plan"]:
                    if cell["stage"] != "G-W":
                        continue
                    adapter, route, repeat = cell["adapters"][0], cell["routes"][0], cell["repeat"]
                    observed = {"read_repeat": repeat, "read_adapter": adapter, "read_route": route,
                        "route_wall_ns": (10 if route == "text" else 20 if route == "hybrid" else 30)*10**9,
                        "route_requests": 1000, "raw_bytes_per_record": (active_bytes(current["reused"][f"g-r-n100000-r{repeat}"]["attempt"])+9005)//9006,
                        "calibration": qualified["costs"][adapter], "serial_storage": storage}
                    legacy = {"components": {"seed_ns": 2*current["reused"]["g1-n100000"]["elapsed_ns"]},
                              "allowed": False, "reason": "original_reseed_forecast"}
                    prediction = forecast_closed_write(cell, observed, 43200*10**9-1, retained, legacy)
                    review["g_table"][cell["id"]] = {"cell": cell, "prediction": prediction}
                review_path = root/"review.json"
                dump(review_path, review)
                Path(current["root"]).mkdir()
                current.update(cost_review_from=str(review_path), cost_review=deepcopy(review))
                with patch("g_freeze.validate_frozen", return_value=current), \
                        patch("g_gpu.inspect_gpu", return_value={"applications": []}), \
                        patch.object(g_closed_costs, "active_bytes", return_value=715233040), \
                        patch("g_serial_storage.derive_model", return_value=storage), \
                        patch("g_resource.resource_sample", return_value={"owned": 0, "task_free": 1024**4,
                            "root_free": 1024**4, "memory_available": 32*1024**3}), \
                        patch.dict(os.environ, {"S30_SOURCE": str(root/"source"), "S30_TASK": str(root)}):
                    prepared = g_closed_entry._preflight(Path(current["root"]))
                self.assertEqual(len(prepared["_closed_predictions"]), 72)
                self.assertEqual(prepared["_reviewed_costs"]["storage_model"], storage)
                self.assertEqual(sum(p["storage"]["closed_bytes"] for p in prepared["_closed_predictions"].values()), 99511043184)
                self.assertEqual(prepared["spec"]["_serial_resource_roots"], {"source": str(root/"source"), "task": str(root)})
