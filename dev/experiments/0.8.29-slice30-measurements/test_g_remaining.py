"""Actual harmless dispatch against retained complete forecasts and literal prior allowance."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from campaign import run_campaign
from g_campaign import make_g_plan, run_g_plan


def full_prediction(total_ns):
    """Return a literal complete synchronous-phase table, never measured timing evidence."""
    names = ["seed_ns", "open_close_ns", "warmup_ns", "natural_ns", "write_ns",
             "readiness_ns", "point_probe_ns", "text_probe_ns"]
    components = dict.fromkeys(names, total_ns // 8)
    components["seed_ns"] += total_ns - sum(components.values())
    return {"allowed": True, "reason": None, "total_ns": total_ns, "new_bytes": 1024,
            "components": components, "cohorts": 1000, "replacement_rows": 1000}


class GRemainingForecastTests(unittest.TestCase):
    def test_whole_forecast_refuses_retry_when_prior_cell_use_leaves_less_than_complete_cost(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        plan = make_g_plan(spec)
        target = next(row for row in plan if row["stage"] == "G-W")
        prediction = full_prediction(600 * 10**9)
        prediction["components"].update(seed_ns=100 * 10**9, open_close_ns=10 * 10**9,
                                        warmup_ns=50 * 10**9, natural_ns=300 * 10**9,
                                        write_ns=20 * 10**9, readiness_ns=40 * 10**9,
                                        point_probe_ns=40 * 10**9, text_probe_ns=40 * 10**9)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marker = root / "unsafe-retry-dispatch"
            code = "\n".join([
                "import json,pathlib,sys",
                "cell=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                f"if cell['id']=={target['id']!r}: pathlib.Path({str(marker)!r}).write_text('launched')",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            command = [sys.executable, "-c", code]
            run_campaign(plan[:12], root, command)
            (root / "g-budget.json").write_text(json.dumps({"schema":"slice30-gd-budget/v1",
                "used_ns":400 * 10**9,"cell_used_ns":{target["id"]:400 * 10**9},"inflight":None}))
            def assess(cell, _previous, _remaining, _retained):
                if cell["id"] != target["id"]:
                    return {"status":"not_run_prerequisite","reason":"bounded_control"}
                return None
            def prepare(cell, _root):
                self.assertEqual(cell["id"], target["id"])
                return {"prospective_prediction": prediction}
            outcomes = run_g_plan(spec, root, command, assess=assess, prepare=prepare)
            self.assertFalse(marker.exists(), "600-second complete forecast dispatched with only500 seconds of cell allowance")
            outcome = next(row for row in outcomes if row["id"] == target["id"])
            self.assertEqual((outcome["status"], outcome["reason"]), ("not_run_cap", "cap_feasibility"))
            self.assertEqual(outcome["prediction"], prediction)
            self.assertEqual(len(outcomes), 84)

    def test_actual_assessment_elapsed_is_removed_before_comparing_complete_forecast(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        prediction = full_prediction(200_000_000)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marker = root / "unsafe-after-assessment"
            (root / "g-budget.json").write_text(json.dumps({"schema":"slice30-gd-budget/v1",
                "used_ns":899_700_000_000,"cell_used_ns":{"g0-smoke":899_700_000_000},"inflight":None}))
            elapsed = []
            def assess(cell, _previous, _remaining, _retained):
                if cell["id"] != "g0-smoke":
                    return {"status":"not_run_prerequisite","reason":"bounded_control"}
                started = time.monotonic_ns()
                while time.monotonic_ns() - started < 200_000_000:
                    pass
                elapsed.append(time.monotonic_ns() - started)
                return None
            def prepare(_cell, _root):
                return {"prospective_prediction":prediction,"actual_assessment_ns":elapsed[0]}
            code = "\n".join([
                "import json,pathlib,sys",
                f"pathlib.Path({str(marker)!r}).write_text('launched')",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            outcomes = run_g_plan(spec, root, [sys.executable, "-c", code], assess=assess, prepare=prepare)
            self.assertFalse(marker.exists(), "complete forecast ignored real assessment wall time")
            self.assertGreaterEqual(elapsed[0], 200_000_000)
            self.assertEqual((outcomes[0]["status"], outcomes[0]["reason"]), ("not_run_cap", "cap_feasibility"))
            self.assertEqual(outcomes[0]["prediction"], prediction)
            self.assertEqual(len(outcomes), 84)


if __name__ == "__main__":
    unittest.main()
