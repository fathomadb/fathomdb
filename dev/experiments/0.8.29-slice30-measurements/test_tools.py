"""Human-specified Slice30 measurement oracles; no database mocking."""

import unittest
import os
import json
from pathlib import Path
import sys
import tempfile
import subprocess

from campaign import make_plan, run_campaign
from control import freeze, run_frozen, summarize
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


class FreezeTests(unittest.TestCase):
    def test_cli_freezes_actual_balanced_cells_and_input_counts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus, queries, cells = [root / name for name in ["corpus.jsonl", "queries.jsonl", "cells.json"]]
            corpus.write_text('"alpha"\n"beta"\n')
            queries.write_text('"alpha"\n')
            cells.write_text(json.dumps([{"name":"tiny", "family":"P1", "n":2,
                                         "adapters":["engine","sdk"], "warmup":1, "samples":2}]))
            out = root / "frozen"
            subprocess.run([sys.executable, str(Path(__file__).with_name("control.py")), "freeze",
                            "--cells", str(cells), "--corpus", str(corpus), "--queries", str(queries),
                            "--binary", sys.executable, "--out", str(out)], check=True)
            self.assertTrue((out / "plan.json").exists(), "freeze command did not publish a cell plan")
            plan = json.loads((out / "plan.json").read_text())
            self.assertEqual([(c["repeat"],c["adapters"]) for c in plan],
                             [(0,["engine","sdk"]),(1,["sdk","engine"])])
            manifest = json.loads((out / "manifest.json").read_text())
            self.assertEqual(manifest["corpus"]["count"], 2)
            self.assertEqual(manifest["queries"]["count"], 1)
            self.assertEqual(manifest["measurement_layer"], "public-direct-text")
            self.assertFalse(manifest["gated"])


class FrozenRunTests(unittest.TestCase):
    def test_frozen_run_executes_each_actual_recording_child(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus, queries, cells = [root / name for name in ["corpus.jsonl", "queries.jsonl", "cells.json"]]
            corpus.write_text('"alpha"\n"beta"\n')
            queries.write_text('"alpha"\n')
            cells.write_text(json.dumps([{"name":"tiny", "family":"P1", "n":2,
                                         "adapters":["engine","sdk"], "warmup":1, "samples":2}]))
            recorder = root / "recording-child"
            calls = root / "calls.jsonl"
            recorder.write_text(f"#!{sys.executable}\n" + "\n".join([
                "import json,pathlib,sys", "c=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                "o=pathlib.Path(sys.argv[2])",
                f"with open({str(calls)!r},'a') as f: f.write(json.dumps(c['repeat'])+'\\n')",
                "(o/'result.json').write_text(json.dumps({'complete':True}))",
            ]) + "\n")
            recorder.chmod(0o755)
            frozen = root / "frozen"
            freeze(cells, corpus, queries, recorder, frozen)
            results = run_frozen(frozen, max_load=None)
            observed = [json.loads(line) for line in calls.read_text().splitlines()] if calls.exists() else []
            self.assertEqual(observed, [0, 1])
            self.assertEqual([row["status"] for row in results], ["complete", "complete"])
            self.assertTrue((frozen / "resources.jsonl").exists())


class SummaryTests(unittest.TestCase):
    def test_summary_keeps_missing_cells_and_observed_samples_separate(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            attempt = root / "results" / "a-r0" / "attempt-0001"
            attempt.mkdir(parents=True)
            plan = [{"id":"a-r0", "family":"P1", "n":2, "samples":4,
                     "adapters":["engine"], "warmup":0},
                    {"id":"a-r1", "family":"P1", "n":2, "samples":4,
                     "adapters":["engine"], "warmup":0}]
            (root / "plan.json").write_text(json.dumps(plan))
            (root / "manifest.json").write_text(json.dumps({"source_sha":"literal sample fixture"}))
            state = [{"id":"a-r0", "attempt":str(attempt), "status":"complete", "exit":0}]
            (root / "results" / "campaign-state.json").write_text(json.dumps(state))
            rows = [{"adapter":"engine", "repeat":0, "query_index":i, "sample_index":i,
                     "ok":True, "elapsed_ns":n * 1000000} for i,n in enumerate([1,2,3,100])]
            (attempt / "samples.jsonl").write_text("".join(json.dumps(row)+"\n" for row in rows))
            (attempt / "warmups.jsonl").write_text("")
            (attempt / "matches.jsonl").write_text('{"query_index":0,"raw_fts_matches":12}\n')
            (attempt / "result.json").write_text(json.dumps({"complete":True,"filtered":False,
                "natural_records":2,"query_count":1,"passes":[{"adapter":"engine","wall_ns":1000000000}]}))
            result = summarize(root)
            self.assertEqual(len(result["cells"]), 2)
            self.assertEqual([cell["status"] for cell in result["cells"]], ["complete","not_run"])
            self.assertEqual(result["cells"][0]["adapters"][0]["statistics"]["successful_latency"]["mean_ms"], 26.5)
            self.assertEqual(result["cells"][0]["match_counts"]["max"], 12)
            self.assertIsNone(result["cells"][0]["adapters"][0]["tracked_distance_ms"])


class ManualRunTests(unittest.TestCase):
    def test_long_cli_requires_manual_opt_in_before_workload(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus, queries, cells = [root / name for name in ["corpus.jsonl","queries.jsonl","cells.json"]]
            corpus.write_text('"alpha"\n"beta"\n')
            queries.write_text('"alpha"\n')
            cells.write_text(json.dumps([{"name":"tiny","family":"P1","n":2,
                "adapters":["engine"],"warmup":0,"samples":1}]))
            child = root / "recording-child"
            child.write_text(f"#!{sys.executable}\nimport pathlib,sys\npathlib.Path(sys.argv[2],'result.json').write_text('''{"complete":true}''')\n")
            child.chmod(0o755)
            frozen = root / "frozen"
            freeze(cells,corpus,queries,child,frozen)
            env = {key:value for key,value in os.environ.items() if key != "AGENT_LONG"}
            result = subprocess.run([sys.executable,str(Path(__file__).with_name("control.py")),
                "run",str(frozen)], env=env, capture_output=True,text=True,timeout=10)
            self.assertEqual(result.returncode, 2)
            self.assertIn("AGENT_LONG=1",result.stderr)
            self.assertFalse((frozen / "results").exists())


if __name__ == "__main__":
    unittest.main()
