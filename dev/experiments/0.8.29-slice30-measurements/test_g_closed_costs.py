"""Literal matched calibration cost table, never a Default/GPU qualification fixture."""

import unittest

from g_closed_costs import calibration_costs


def cost_record(adapter, batch, ratio, write_ns, point, text, ready):
    """Representative hand-specified maxima; population qualification is a separate layer."""
    rows = (1000 // ratio) * batch
    copied = 715223040
    return {"cell": {"id": f"g-cal-{adapter}-b{batch}-s{ratio}", "n": 100000, "adapters": [adapter],
                     "batch_size": batch, "searches_per_batch": ratio, "samples": 1000},
        "peak_store_bytes": copied + 4096 * rows, "retained_bytes": copied + 10000,
        "actual": {"preparation": {"validated": True, "copy_ns": 2*10**9,
                                  "validation_ns": 10**9, "copied_bytes": copied},
            "steps": [{"host_wall_ns": 10*10**9, "observation_max_wall_ns": 20_000_000,
                "result": {"passes": [{"wall_ns": 5*10**9}], "warmups": [{"elapsed_ns": 10**9}],
                    "batches": [{"rows": batch, "write_ns": write_ns,
                                 "adjacent_readiness": {"available": True, "elapsed_ns": ready}}],
                    "probes": [{"point_probe": {"elapsed_ns": point}, "text_probe": {"elapsed_ns": text}}],
                    "samples": []}}]}}


class GClosedCostExtractionTests(unittest.TestCase):
    def test_actual_matched_dense_sparse_fields_supply_conservative_bounds(self):
        records = [cost_record("engine", 10, 1, 40_000_000, 1_000_000, 2_000_000, 3_000_000),
                   cost_record("sdk", 10, 1, 900_000_000, 900_000_000, 900_000_000, 900_000_000),
                   cost_record("engine", 1, 100, 5_000_000, 2_000_000, 3_000_000, 4_000_000),
                   cost_record("sdk", 1, 100, 900_000_000, 900_000_000, 900_000_000, 900_000_000)]
        actual = calibration_costs(records, "engine")
        self.assertEqual(actual.get("copy_ns"), 2*10**9, "qualified copy costs were not extracted")
        self.assertEqual(actual["validation_ns"], 10**9)
        self.assertEqual(actual["copy_bytes"], 715223040)
        self.assertEqual(actual["driver_remainder_ns"], 4*10**9)
        self.assertEqual(actual["point_probe_max_ns"], 2_000_000)
        self.assertEqual(actual["text_probe_max_ns"], 3_000_000)
        self.assertEqual(actual["readiness_max_ns"], 4_000_000)
        self.assertEqual(actual["write_row_max_ns"], 5_000_000)
        self.assertEqual(actual["physical_growth_per_row"], 4096)
        self.assertEqual(actual["fixed_bytes"], 10000)
        self.assertEqual(actual["observer_tail_max_ns"], 20_000_000)
        self.assertEqual(actual["cells"], ["g-cal-engine-b10-s1", "g-cal-engine-b1-s100"])
        self.assertEqual(actual["adapter"], "engine")
        self.assertEqual(actual["n"], 100000)
