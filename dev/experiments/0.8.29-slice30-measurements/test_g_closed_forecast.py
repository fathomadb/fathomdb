"""Hand-computed conservative closed-copy costs; no smaller-fixture substitution."""

from copy import deepcopy
import unittest

from g_campaign import make_g_plan
from g_forecast import forecast_write
from test_g_closed_plan import SPEC


def observed():
    return {"seed_wall_ns": 657935962544, "seed_bytes": 730417530, "seed_write_ns": 20*10**9,
        "seed_rows": 100000, "route_wall_ns": 30*10**9, "route_requests": 1000,
        "open_ns": 10**9, "close_ns": 10_000_000, "text_max_ns": 500_000_000,
        "point_probe_max_ns": 1_000_000, "text_probe_max_ns": 2_000_000,
        "readiness_max_ns": 3_000_000, "write_row_max_ns": 4_000_000,
        "raw_bytes_per_record": 1024, "read_repeat": 0, "read_adapter": "engine", "read_route": "text",
        "calibration": {"n": 100000, "adapter": "engine",
            "cells": ["g-cal-engine-b10-s1", "g-cal-engine-b1-s100"],
            "copy_ns": 2*10**9, "validation_ns": 10**9, "copy_bytes": 715223040,
            "driver_remainder_ns": 10**9, "point_probe_max_ns": 1_000_000,
            "text_probe_max_ns": 2_000_000, "readiness_max_ns": 3_000_000,
            "write_row_max_ns": 4_000_000, "observer_tail_max_ns": 20_000_000,
            "physical_growth_per_row": 4096, "fixed_bytes": 1024}}


class GClosedForecastTests(unittest.TestCase):
    def test_copy_policy_keeps_2x_and_actual_100k_costs_instead_of_reseeding(self):
        cell = next(row for row in make_g_plan(dict(SPEC, preparation="closed-copy-v1"))
                    if row["id"] == "g-w-b1-s100-engine-text-r0")
        prediction = forecast_write(cell, observed(), 14400*10**9, 1903142018)
        self.assertTrue(prediction["allowed"], "closed copy still inherits the failed reseed-only forecast")
        self.assertEqual(prediction["components"], {
            "copy_validation_ns": 6*10**9, "driver_remainder_ns": 2*10**9,
            "warmup_ns": 30*10**9, "natural_ns": 60*10**9, "write_ns": 80_000_000,
            "readiness_ns": 66_000_000, "point_probe_ns": 20_000_000,
            "text_probe_ns": 40_000_000, "observer_tail_ns": 40_000_000})
        self.assertEqual(prediction["total_ns"], 98_246_000_000)
        self.assertEqual(prediction["new_bytes"], 718440448)
        self.assertFalse(prediction["legacy_forecast"]["allowed"])
        self.assertGreater(prediction["legacy_forecast"]["components"]["seed_ns"], 900*10**9)
        short = forecast_write(cell, observed(), prediction["total_ns"]-1, 0)
        self.assertFalse(short["allowed"])
        self.assertEqual(short["reason"], "shared_time_forecast")
        full = forecast_write(cell, observed(), 14400*10**9, 16*1024**3-prediction["new_bytes"]+1)
        self.assertFalse(full["allowed"])
        self.assertEqual(full["reason"], "retained_storage_forecast")
        wrong_repeat = deepcopy(observed())
        wrong_repeat["read_repeat"] = 1
        missing = forecast_write(cell, wrong_repeat, 14400*10**9, 0)
        self.assertFalse(missing["allowed"], "R1/R0 matching cost was silently substituted")
        self.assertEqual(missing["reason"], "calibration_cost_unavailable")
