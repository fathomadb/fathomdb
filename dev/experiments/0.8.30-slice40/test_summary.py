"""Check the fixed study's metrics and pair denominators."""

import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from summarize_study import metrics, pair_summary, population, summarize


class MetricsTest(unittest.TestCase):
    def test_only_successes_contribute_latency_and_useful_throughput(self):
        rows = [
            {"ok": True, "elapsed_ns": 10_000_000, "bodies": ["é"]},
            {"ok": True, "elapsed_ns": 30_000_000, "bodies": ["a"]},
            {"ok": False, "elapsed_ns": 100_000_000, "error_kind": "OverloadedError"},
            {
                "ok": False,
                "elapsed_ns": 120_000_000,
                "error_kind": "EmbedderError",
                "error": "Engine(Embedder(Timeout))",
            },
            {"ok": False, "elapsed_ns": 120_000_000, "error_kind": "StorageError"},
        ]
        result = metrics(rows, 6, 2_000_000_000)
        self.assertEqual(result["latency_ms"], {"p50": 10, "p95": 30, "p99": 30})
        self.assertEqual(result["useful_qps"], 1)
        self.assertEqual(result["refusals"], 1)
        self.assertEqual(result["timeouts"], 1)
        self.assertEqual(result["errors"], 1)
        self.assertEqual(result["missing"], 1)
        self.assertEqual(result["returned_utf8_body_bytes"], 3)

    def test_repeated_indices_cannot_stand_in_for_missing_samples(self):
        data = {"samples": [{"route": "text", "sample_index": 0}] * 2}
        with self.assertRaisesRegex(ValueError, "count or order"):
            population(data, "samples", ["text"], 2)

    def test_partial_population_preserves_ordered_completed_samples(self):
        rows = [
            {
                "route": "hybrid",
                "sample_index": 0,
                "query_index": 0,
                "ok": True,
                "elapsed_ns": 1000,
                "degradations": "[EmbedderTimeout]",
            }
        ]
        retained = population({"samples": rows}, "samples", ["hybrid"], 2, partial=True)
        result = metrics(retained[0], 2)
        self.assertEqual(result["successful"], 1)
        self.assertEqual(result["degraded_successful"], 1)
        self.assertEqual(result["missing"], 1)
        self.assertEqual(result["timeouts"], 0)
        rows[0]["query_index"] = 1
        with self.assertRaisesRegex(ValueError, "count or order"):
            population({"samples": rows}, "samples", ["hybrid"], 2, partial=True)

    def test_interrupted_output_retains_preceding_complete_calls(self):
        with TemporaryDirectory() as scratch:
            root = Path(scratch)
            cell = root / "read-0-hybrid"
            native = cell / "native"
            native.mkdir(parents=True)
            (cell / "config.json").write_text(
                json.dumps(
                    {
                        "family": "G-R",
                        "routes": ["hybrid"],
                        "samples": 2,
                        "warmup": 0,
                        "observe": False,
                        "record_first_call": False,
                    }
                )
            )
            (cell / "attempt.json").write_text(
                json.dumps({"exit": -9, "watchdog": True})
            )
            (native / "result.json").write_text('{"samples":[')
            finished = {
                "route": "hybrid",
                "sample_index": 0,
                "query_index": 0,
                "ok": True,
                "elapsed_ns": 1000,
            }
            (native / "samples.jsonl").write_bytes(
                (json.dumps(finished) + '\n{"route":').encode()
            )
            result = summarize(root)["cells"]["read-0-hybrid"]
            self.assertFalse(result["complete"])
            self.assertEqual(result["routes"]["hybrid"]["successful"], 1)
            self.assertEqual(result["routes"]["hybrid"]["missing"], 1)
            self.assertEqual(result["routes"]["hybrid"]["timeouts"], 0)
            self.assertEqual(len(result["interrupted_outputs"]), 2)

    def test_relative_improvement_is_computed_within_each_block(self):
        def cell(p50, throughput, cpu, rss):
            return {
                "latency_ms": {"p50": p50},
                "useful_qps": throughput,
                "cpu_seconds_per_success": cpu,
                "peak_rss_kib": rss,
            }

        a = [cell(100, 10, 10, 200), cell(500, 10, 10, 100), cell(200, 10, 10, 160)]
        b = [cell(50, 9, 8, 200), cell(400, 12, 12, 105), cell(180, 10, 9, 155)]
        paired = pair_summary(a, b)
        self.assertEqual(paired["paired_observed_bounds"], [0.1, 0.5])
        self.assertEqual(paired["median_relative_p50_improvement"], 0.2)
        self.assertEqual(paired["median_absolute_p50_improvement_ms"], 50)
        self.assertEqual(paired["median_useful_qps_ratio"], 1)
        self.assertEqual(paired["median_cpu_per_success_ratio"], 0.9)
        self.assertEqual(paired["median_peak_rss_ratio"], 1)


if __name__ == "__main__":
    unittest.main()
