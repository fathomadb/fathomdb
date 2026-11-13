"""Literal prospective cost tables plus actual harmless forbidden-child witness."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from g_campaign import make_g_plan
from g_forecast import forecast_write


class GWholeWriteForecastTests(unittest.TestCase):
    def test_seed_that_fits_does_not_authorize_whole_workload_that_exceeds_cell_cap(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        cell = next(row for row in make_g_plan(spec) if row["stage"] == "G-W")
        observed = {"seed_wall_ns": 350 * 10**9, "seed_bytes": 1024**3,
                    "seed_write_ns": 50 * 10**9, "seed_rows": 100000,
                    "route_wall_ns": 100 * 10**9, "route_requests": 1000,
                    "open_ns": 10**9, "close_ns": 10**9,
                    "text_max_ns": 200_000_000,
                    "point_probe_max_ns": 1_000_000, "text_probe_max_ns": 1_000_000,
                    "readiness_max_ns": 1_000_000, "write_row_max_ns": 1_000_000,
                    "raw_bytes_per_record": 1024}
        prediction = forecast_write(cell, observed, 14400 * 10**9, 0)
        with tempfile.TemporaryDirectory() as temporary:
            marker = Path(temporary) / "forbidden-whole-cell"
            if prediction["allowed"]:
                subprocess.run([sys.executable, "-c", f"from pathlib import Path; Path({str(marker)!r}).write_text('launched')"],
                               check=True)
            self.assertFalse(marker.exists(), "seed-only feasibility launched an over-budget whole G-W cell")
        self.assertFalse(prediction["allowed"])
        self.assertGreater(prediction["total_ns"], 900 * 10**9)
        self.assertEqual(set(prediction["components"]),
                         {"seed_ns", "open_close_ns", "warmup_ns", "natural_ns", "write_ns",
                          "readiness_ns", "point_probe_ns", "text_probe_ns"})
        self.assertEqual(prediction["cohorts"], 1000)
        self.assertEqual(prediction["replacement_rows"], 1000)
        self.assertEqual(prediction["components"]["warmup_ns"], 100 * 10**9)
        self.assertEqual(prediction["components"]["natural_ns"], 200 * 10**9)
        self.assertEqual(prediction["total_ns"], sum(prediction["components"].values()))


if __name__ == "__main__":
    unittest.main()

class GWriteRemainingFitTests(unittest.TestCase):
    def test_whole_estimate_checks_shared_remaining_raw_headroom_and_real_cohort_counts(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        cell = next(row for row in make_g_plan(spec) if row["stage"] == "G-W"
                    and row["batch_size"] == 10 and row["searches_per_batch"] == 100)
        observed = {"seed_wall_ns": 10**9, "seed_bytes": 1024**2,
                    "seed_write_ns": 10**8, "seed_rows": 100000,
                    "route_wall_ns": 10**9, "route_requests": 1000,
                    "open_ns": 1_000_000, "close_ns": 1_000_000,
                    "text_max_ns": 1_000_000, "point_probe_max_ns": 1_000_000,
                    "text_probe_max_ns": 1_000_000, "readiness_max_ns": 1_000_000,
                    "write_row_max_ns": 1_000_000, "raw_bytes_per_record": 1024}
        fit = forecast_write(cell, observed, 14400 * 10**9, 0)
        self.assertTrue(fit["allowed"])
        self.assertEqual(fit.get("cohorts"), 10, "forecast did not count actual s100 cohorts")
        self.assertEqual(fit["replacement_rows"], 100)
        remaining = forecast_write(cell, observed, fit["total_ns"] - 1, 0)
        self.assertFalse(remaining["allowed"], "whole G-W estimate exceeded shared remaining allocation")
        storage = forecast_write(cell, observed, 14400 * 10**9,
                                 16 * 1024**3 - 2 * observed["seed_bytes"] - 1)
        self.assertFalse(storage["allowed"], "whole G-W estimate omitted retained raw growth")

    def test_missing_observed_costs_refuse_instead_of_inventing_zero_cost(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        cell = next(row for row in make_g_plan(spec) if row["stage"] == "G-W")
        result = forecast_write(cell, {"seed_wall_ns": 10**9, "seed_bytes": 1024},
                                14400 * 10**9, 0)
        self.assertFalse(result["allowed"], "missing workload observations were treated as free")
        self.assertEqual(result["reason"], "cost_observation_unavailable")
