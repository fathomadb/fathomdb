"""Literal prospective R1 cost table preserves its fresh-copy methodological distinction."""

from copy import deepcopy
import unittest

from g_campaign import make_g_plan
from g_closed_forecast import forecast_closed_read
from test_g_closed_forecast import observed
from test_g_closed_plan import SPEC


class GClosedReadForecastTests(unittest.TestCase):
    def test_reverse_read_forecast_adds_copy_to2x_entire_ordinary_r0(self):
        cell = next(row for row in make_g_plan(dict(SPEC, preparation="closed-copy-v1"))
                    if row["id"] == "g-r-n100000-r1")
        engine = deepcopy(observed()["calibration"])
        engine["fixed_bytes"] = 10000
        sdk = dict(engine, adapter="sdk", cells=["g-cal-sdk-b10-s1", "g-cal-sdk-b1-s100"])
        facts = {"elapsed_ns": 10*10**9, "retained_bytes": 1024, "n": 100000, "repeat": 0}
        result = forecast_closed_read(cell, facts, {"engine": engine, "sdk": sdk},
                                      14400*10**9, 1903142018, 1315871925088)
        self.assertTrue(result["allowed"], "matched copy/R0 evidence did not admit the prospective bounded R1")
        self.assertEqual(result["components"], {"copy_validation_ns": 6*10**9, "ordinary_r0_ns": 20*10**9})
        self.assertEqual(result["total_ns"], 26*10**9)
        self.assertEqual(result["new_bytes"], 715245088)
        self.assertEqual(result["legacy_seed_ns"], 1315871925088)
        self.assertFalse(result["legacy_seed_time_feasible_900s"])
        short = forecast_closed_read(cell, facts, {"engine": engine, "sdk": sdk},
                                     26*10**9-1, 0, 1315871925088)
        self.assertFalse(short["allowed"])
        self.assertEqual(short["reason"], "shared_time_forecast")
