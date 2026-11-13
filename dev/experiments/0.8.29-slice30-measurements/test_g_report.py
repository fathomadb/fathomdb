"""Literal diagnostic report populations, never substituted G measurement evidence."""

import json
from pathlib import Path
import tempfile
import unittest

from g_campaign import make_g_plan
from g_report import summarize


class GReportTests(unittest.TestCase):
    def test_all_obligations_retained_and_fifty_query_diagnostic_has_no_percentile_claim(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        plan = make_g_plan(spec)
        cell = next(row for row in plan if row["id"] == "g-o-natural-r0-off")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "manifest.json").write_text(json.dumps({"plan": plan}))
            results = root / "results"
            attempt = results / cell["id"] / "attempt-0001"
            attempt.mkdir(parents=True)
            rows = [{"adapter": "engine", "route": "text", "ok": True, "query_index": index,
                     "sample_index": index, "repeat": 0, "elapsed_ns": (index + 1) * 10**6} for index in range(50)]
            result = {"complete": False, "steps": [{"result": {"samples": rows,
                      "passes": [{"adapter": "engine", "route": "text", "wall_ns": 50 * 10**9}]}}]}
            (attempt / "result.json").write_text(json.dumps(result))
            (results / "g-outcomes.json").write_text(json.dumps([{"id": cell["id"], "status": "failed", "attempt": str(attempt)}]))
            report = summarize(root)
            self.assertEqual(len(report["cells"]), 84)
            self.assertFalse(report["complete"])
            selected = next(row for row in report["cells"] if row["id"] == cell["id"])
            statistics = selected["populations"][0]["statistics"]
            self.assertEqual(statistics["requests"], 50)
            self.assertEqual(statistics["successful_latency"]["mean_ms"], 25.5)
            self.assertNotIn("p50_ms", statistics["successful_latency"], "50-query diagnostic gained a percentile claim")
            self.assertNotIn("p99_ms", statistics["successful_latency"])
            self.assertEqual(sum(row["status"] == "not_run" for row in report["cells"]), 83)


if __name__ == "__main__":
    unittest.main()
