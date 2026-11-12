"""Human-specified Slice30 measurement oracles; no database mocking."""

import unittest
import json
from pathlib import Path
import sys
import tempfile

from campaign import make_plan, run_campaign
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


class RunnerTests(unittest.TestCase):
    def test_balanced_order_runs_real_children_in_distinct_attempts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            recorder = root / "calls.jsonl"
            code = (
                "import json,pathlib,sys; "
                "c=json.loads(pathlib.Path(sys.argv[1]).read_text()); "
                "o=pathlib.Path(sys.argv[2]); "
                f"f=open({str(recorder)!r},'a'); "
                "f.write(json.dumps([c['name'],c['repeat'],str(o)])+'\\n');f.close(); "
                "(o/'run.txt').write_text(c['id']); "
                "(o/'result.json').write_text(json.dumps({'complete':True}))"
            )
            plan = make_plan([{"name": "a"}, {"name": "b"}], 2)
            run_campaign(plan, root / "results", [sys.executable, "-c", code])
            calls = [json.loads(line) for line in recorder.read_text().splitlines()] \
                if recorder.exists() else []
            self.assertEqual([row[:2] for row in calls],
                             [["a", 0], ["b", 0], ["b", 1], ["a", 1]])
            self.assertEqual(len({row[2] for row in calls}), 4)
            self.assertTrue(all((Path(row[2]) / "run.txt").exists() for row in calls))

    def test_resume_keeps_complete_cells_and_restarts_failed_attempt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "calls.jsonl"
            failed = root / "failed-once"
            code = "\n".join([
                "import json,pathlib,sys",
                "c=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                "o=pathlib.Path(sys.argv[2])",
                f"with open({str(calls)!r},'a') as f: f.write(json.dumps([c['name'],str(o)])+'\\n')",
                f"flag=pathlib.Path({str(failed)!r})",
                "if c['name']=='b' and not flag.exists(): flag.write_text('failed'); sys.exit(7)",
                "(o/'result.json').write_text(json.dumps({'complete':True}))",
            ])
            plan = make_plan([{"name": "a"}, {"name": "b"}], 1)
            command = [sys.executable, "-c", code]
            first = run_campaign(plan, root / "results", command)
            second = run_campaign(plan, root / "results", command)
            observed = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual([row[0] for row in observed], ["a", "b", "b"])
            self.assertNotEqual(observed[1][1], observed[2][1])
            self.assertEqual(first[1]["exit"], 7)
            self.assertEqual([row["status"] for row in second], ["complete", "complete"])
            self.assertTrue((Path(observed[1][1]) / "child.log").exists())


if __name__ == "__main__":
    unittest.main()
