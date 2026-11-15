"""Hand-computed serial storage accounting; no performance or GPU execution."""

from copy import deepcopy
import unittest

from g_campaign import make_g_plan
from g_closed_forecast import forecast_closed_write
from test_g_closed_forecast import observed
from test_g_closed_plan import SPEC

SCHEMA = "slice30-g-serial-retained-live/v1"


def model():
    """Return the six retained observations frozen in the accepted design."""
    rows = [
        ("g-r-n100000-r0", 311140605, 9006, 3375, 0, 0, 0, 0),
        ("g-r-n100000-r1", 311137399, 9006, 13146, 715223040, 0, 0, 32768),
        ("g-cal-engine-b10-s1", 210171059, 12500, 10684, 715223040, 82223104, 10000, 127771488),
        ("g-cal-sdk-b10-s1", 209999403, 12500, 10594, 715223040, 82223104, 10000, 113950872),
        ("g-cal-engine-b1-s100", 80200683, 1520, 10711, 715223040, 98304, 10, 2381200),
        ("g-cal-sdk-b1-s100", 80185212, 1520, 10621, 715223040, 98304, 10, 2381200),
    ]
    return {"schema": SCHEMA, "factor": 2, "baseline_bytes": 715223040,
            "raw_bytes_per_record": 52764, "residual_bytes": 13146,
            "closed_growth_per_row": 9831, "live_growth_per_row": 238120,
            "basis": {"compiler_inputs": "protocol-inputs", "r0": {"root": "protocol-original"},
                      "r1": {"root": "protocol-r1"}, "calibration": {"root": "protocol-calibration"},
                      "records": [dict(zip(("id", "serialized_bytes", "record_count", "residual_bytes",
                                           "baseline_bytes", "closed_growth_bytes", "replacement_rows",
                                           "live_growth_bytes"), row, strict=True)) for row in rows]}}


def prediction(cell):
    value = observed()
    adapter, route, repeat = cell["adapters"][0], cell["routes"][0], cell["repeat"]
    value.update(read_adapter=adapter, read_route=route, read_repeat=repeat, serial_storage=model())
    value["calibration"].update(adapter=adapter,
        cells=[f"g-cal-{adapter}-b10-s1", f"g-cal-{adapter}-b1-s100"])
    return forecast_closed_write(cell, value, 43200*10**9, 0, {"allowed": False})


class GSerialStorageTests(unittest.TestCase):
    def test_sparse_total_envelopes_preserve_all_four_physical_serializations(self):
        cell = next(c for c in make_g_plan(dict(SPEC, preparation="closed-copy-v1"))
                    if c["id"] == "g-w-b1-s100-engine-text-r0")
        actual = prediction(cell)
        self.assertEqual(actual.get("new_bytes"), 881469572,
                         "write storage still repeats the baseline/raw overlap instead of the named total live envelope")
        self.assertEqual(actual.get("storage"), {"schema": SCHEMA, "closed_bytes": 876903792,
            "live_bytes": 881469572, "live_excess_bytes": 4565780})
        self.assertEqual(actual["total_ns"], 98246000000, "storage policy altered a time cost")

    def test_complete_72_table_has_comparable_closed_and_live_totals(self):
        cells = [c for c in make_g_plan(dict(SPEC, preparation="closed-copy-v1")) if c["stage"] == "G-W"]
        values = [prediction(c) for c in cells]
        self.assertEqual(len(values), 72)
        self.assertEqual(sum(v.get("storage", {}).get("closed_bytes", 0) for v in values), 99511043184,
                         "closed retained cases still lack their separate total envelope")
        self.assertEqual(sum(v["new_bytes"] for v in values), 166408851744)
        self.assertEqual(max(v["storage"]["live_excess_bytes"] for v in values), 4565780000)
        self.assertEqual(7775436266 + 1073741824 + sum(v["storage"]["closed_bytes"] for v in values)
                         + max(v["storage"]["live_excess_bytes"] for v in values), 112926001274)

    def test_named_storage_model_never_changes_legacy_time_components(self):
        cell = next(c for c in make_g_plan(dict(SPEC, preparation="closed-copy-v1")) if c["stage"] == "G-W")
        with_model = prediction(cell)
        legacy_observed = deepcopy(observed())
        legacy = forecast_closed_write(cell, legacy_observed, 43200*10**9, 0, {"allowed": False})
        self.assertEqual(with_model["components"], legacy["components"])
        self.assertEqual(with_model["total_ns"], legacy["total_ns"])
