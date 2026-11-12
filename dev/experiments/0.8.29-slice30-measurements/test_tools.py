"""Human-specified Slice30 measurement oracles; no database mocking."""

import unittest

from converter import convert
from metrics import aggregate, distribution, paired_difference


class ConversionTests(unittest.TestCase):
    def test_prefix_selection_keeps_first_seen_text_and_all_questions(self):
        data = [
            {"question": "Q one", "haystack_session_ids": ["a"],
             "haystack_sessions": [[{"content": " alpha "}, {"content": ""},
                                     {"content": "beta"}]]},
            {"question": "Q two", "haystack_session_ids": ["a", "b"],
             "haystack_sessions": [[{"content": "ignored repeated session"}],
                                    [{"content": "beta"}, {"content": "gamma"}]]},
        ]
        result = convert(data, 2)
        self.assertEqual(result["bodies"], ["alpha", "beta"])
        self.assertEqual(result["queries"], ["Q one", "Q two"])
        self.assertEqual(result["sessions"], 2)
        self.assertEqual(result["total_unique_turns"], 3)
        self.assertEqual(result["selected_count"], 2)

    def test_misaligned_session_arrays_are_rejected(self):
        data = [{"question": "Q", "haystack_session_ids": ["a", "b"],
                 "haystack_sessions": [[{"content": "alpha"}]]}]
        with self.assertRaisesRegex(ValueError, "session.*length"):
            convert(data, 2)


class MetricTests(unittest.TestCase):
    def test_nearest_rank_percentiles_and_arithmetic_mean(self):
        result = distribution([1000000, 2000000, 3000000, 100000000])
        self.assertEqual(result.get("p50_ms"), 2.0)
        self.assertEqual(result.get("p99_ms"), 100.0)
        self.assertEqual(result.get("mean_ms"), 26.5)
        self.assertEqual(result["count"], 4)
        self.assertEqual(distribution([]),
                         {"count": 0, "p50_ms": None, "p99_ms": None, "mean_ms": None})

    def test_errors_remain_in_throughput_denominator(self):
        rows = [{"ok": True, "elapsed_ns": n * 1000000} for n in [1, 2, 3, 100]]
        rows.append({"ok": False, "elapsed_ns": 200000000, "error": "Closing"})
        result = aggregate(rows, 1000000000)
        self.assertEqual(result.get("ok"), 4)
        self.assertEqual(result.get("errors"), {"Closing": 1})
        self.assertEqual(result.get("throughput_all_per_s"), 5.0)
        self.assertEqual(result.get("successful_latency", {}).get("mean_ms"), 26.5)
        self.assertEqual(result.get("error_latency", {}).get("p50_ms"), 200.0)

    def test_adapter_differences_pair_queries_within_repeats(self):
        rows = [{"adapter": a, "repeat": r, "query_index": q,
                 "ok": True, "elapsed_ns": n * 1000000}
                for r in [0, 1] for a, q, n in
                [("engine", 0, 10), ("engine", 1, 20), ("sdk", 0, 13), ("sdk", 1, 24)]]
        result = paired_difference(rows)
        self.assertEqual(result.get("pairs"), 4)
        self.assertEqual(result.get("distinct_queries"), 2)
        self.assertEqual(result.get("sdk_minus_engine_mean_ms"), 3.5)
        self.assertEqual(result.get("bootstrap95_ms"), [3.0, 4.0])


if __name__ == "__main__":
    unittest.main()
